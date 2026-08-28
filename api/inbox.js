const PREFIX = "Holeshot HUD ·";
const MAX_REPLY = 4000;

function authorized(req) {
  const secret = process.env.FEEDBACK_INBOX_SECRET;
  if (!secret) return false;
  const header = String(req.headers["x-inbox-secret"] || "");
  const query = String((req.query && req.query.secret) || "");
  return header === secret || query === secret;
}

function ghHeaders(token) {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "User-Agent": "Holeshot-HUD-inbox",
    "X-GitHub-Api-Version": "2022-11-28",
  };
}

function clipRaw(s, n) {
  const t = String(s || "");
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function parseDesc(description) {
  const rest = String(description || "").replace(PREFIX, "").trim();
  const sep = rest.indexOf("·");
  if (sep < 0) {
    return { kind: "other", summary: rest, replied: false, waiting: false, archived: false, since: "" };
  }
  let kind = rest.slice(0, sep).trim();
  let summary = rest.slice(sep + 1).trim();
  let archived = false;
  if (kind === "archived") {
    archived = true;
    const sep2 = summary.indexOf("·");
    if (sep2 >= 0) {
      kind = summary.slice(0, sep2).trim();
      summary = summary.slice(sep2 + 1).trim();
    } else {
      kind = "other";
    }
  }
  let waiting = false;
  let replied = false;
  let since = "";
  while (true) {
    if (summary.startsWith("[waiting]")) {
      waiting = true;
      summary = summary.replace(/^\[waiting\]\s*/, "");
      continue;
    }
    if (summary.startsWith("[replied]")) {
      replied = true;
      summary = summary.replace(/^\[replied\]\s*/, "");
      continue;
    }
    const m = summary.match(/^\[since\s+([^\]]+)\]\s*/);
    if (m) {
      since = m[1].trim();
      summary = summary.slice(m[0].length);
      continue;
    }
    break;
  }
  return { kind, summary, replied, waiting, archived, since };
}

function encodeDesc({ kind, summary, replied, waiting, archived, since }) {
  const parts = [];
  if (archived) parts.push("archived");
  parts.push(kind || "other");
  let clean = String(summary || "")
    .replace(/^\[waiting\]\s*/, "")
    .replace(/^\[replied\]\s*/, "")
    .replace(/^\[since\s+[^\]]+\]\s*/, "");
  const tags = [];
  if (waiting) tags.push("[waiting]");
  else if (replied) tags.push("[replied]");
  if (since) tags.push(`[since ${since}]`);
  parts.push(`${tags.join(" ")}${tags.length ? " " : ""}${clean}`.trim());
  return `${PREFIX} ${parts.join(" · ")}`;
}

function isOurs(gist) {
  return String(gist.description || "").startsWith(PREFIX);
}

function parseFeedback(text) {
  try {
    const data = JSON.parse(text);
    if (data && typeof data === "object") return data;
  } catch {
    /* ignore */
  }
  return { message: String(text || "") };
}

function readBody(req) {
  if (req.body == null) return {};
  if (typeof req.body === "object") return req.body;
  try {
    return JSON.parse(req.body);
  } catch {
    return {};
  }
}

async function listGists(token) {
  const items = [];
  for (let page = 1; page <= 5; page += 1) {
    const gh = await fetch(`https://api.github.com/gists?per_page=100&page=${page}`, {
      headers: ghHeaders(token),
    });
    if (!gh.ok) {
      const err = await gh.text();
      throw new Error(err.slice(0, 400));
    }
    const batch = await gh.json();
    if (!Array.isArray(batch) || batch.length === 0) break;
    for (const gist of batch) {
      if (!isOurs(gist)) continue;
      const { kind, summary, replied, waiting, archived, since } = parseDesc(gist.description);
      items.push({
        id: gist.id,
        kind,
        summary,
        replied,
        waiting,
        archived,
        since,
        at: gist.created_at,
        url: gist.html_url,
        files: Object.keys(gist.files || {}),
      });
    }
    if (batch.length < 100) break;
  }
  items.sort((a, b) => String(b.at).localeCompare(String(a.at)));
  return items;
}

async function readGist(token, id) {
  const gh = await fetch(`https://api.github.com/gists/${encodeURIComponent(id)}`, {
    headers: ghHeaders(token),
  });
  if (!gh.ok) return null;
  const gist = await gh.json();
  if (!isOurs(gist)) return null;
  const files = {};
  for (const [name, file] of Object.entries(gist.files || {})) {
    const text = String(file.content || "");
    files[name] = name.endsWith(".jsonl") && text.length > 80_000 ? `${text.slice(0, 80_000)}\n…` : text;
  }
  const parsed = parseDesc(gist.description);
  return {
    id: gist.id,
    description: gist.description,
    at: gist.created_at,
    url: gist.html_url,
    files,
    kind: parsed.kind,
    summary: parsed.summary,
    replied: parsed.replied,
    waiting: parsed.waiting,
    archived: parsed.archived,
    since: parsed.since,
  };
}

async function patchGist(token, id, { description, feedback }) {
  const files = {};
  if (feedback) {
    files["feedback.json"] = { content: JSON.stringify(feedback, null, 2) };
  }
  const body = { files };
  if (description) body.description = description;
  const gh = await fetch(`https://api.github.com/gists/${encodeURIComponent(id)}`, {
    method: "PATCH",
    headers: ghHeaders(token),
    body: JSON.stringify(body),
  });
  if (!gh.ok) {
    const err = await gh.text();
    throw new Error(err.slice(0, 400));
  }
  return gh.json();
}

function isInstaller(name) {
  return /setup\.exe$/i.test(String(name || ""));
}

function assetDownloads(rel) {
  let n = 0;
  for (const asset of rel.assets || []) {
    if (isInstaller(asset.name)) n += Number(asset.download_count) || 0;
  }
  return n;
}

async function installerDownloads(token) {
  try {
    let total = 0;
    let latest = null;
    for (let page = 1; page <= 5; page += 1) {
      const gh = await fetch(
        `https://api.github.com/repos/LeadingTrendTechnologies/HoleshotHUD/releases?per_page=100&page=${page}`,
        { headers: ghHeaders(token) }
      );
      if (!gh.ok) {
        if (page === 1) return null;
        break;
      }
      const batch = await gh.json();
      if (!Array.isArray(batch) || batch.length === 0) break;
      for (const rel of batch) {
        if (rel.draft) continue;
        const n = assetDownloads(rel);
        total += n;
        if (!latest) latest = { tag: rel.tag_name || "", installer: n };
      }
      if (batch.length < 100) break;
    }
    return { installer: total, latest };
  } catch {
    return null;
  }
}

function clipThread(thread) {
  const out = [];
  for (const item of Array.isArray(thread) ? thread : []) {
    const from = item && item.from === "dev" ? "dev" : item && item.from === "user" ? "user" : "";
    const text = clipRaw(item && item.text, MAX_REPLY).trim();
    if (!from || !text) continue;
    out.push({ from, text, at: item.at || null });
    if (out.length >= 40) break;
  }
  return out;
}

function ensureThread(feedback) {
  let thread = clipThread(feedback.thread);
  if (!thread.length && typeof feedback.reply === "string" && feedback.reply.trim()) {
    thread = [
      {
        from: "dev",
        text: clipRaw(feedback.reply, MAX_REPLY).trim(),
        at: feedback.replied_at || null,
      },
    ];
  }
  feedback.thread = thread;
  return thread;
}

async function applyInboxAction(token, data) {
  const id = String(data.id || "").trim();
  if (!/^[A-Za-z0-9]{8,64}$/.test(id)) {
    return { status: 400, body: { error: "Missing ticket id." } };
  }
  const current = await readGist(token, id);
  if (!current) {
    return { status: 404, body: { error: "Not found." } };
  }
  const feedback = parseFeedback(current.files["feedback.json"] || "");
  const parsed = parseDesc(current.description);
  const kind = feedback.kind || parsed.kind || "other";
  const summary = parsed.summary || clipRaw(feedback.message, 72) || kind;
  const replyText = typeof data.reply === "string" ? clipRaw(data.reply, MAX_REPLY).trim() : "";
  const archive = Boolean(data.archive);
  if (!replyText && !archive) {
    return { status: 400, body: { error: "Write a reply first." } };
  }
  const thread = ensureThread(feedback);
  if (replyText) {
    if (thread.length >= 40) {
      return { status: 400, body: { error: "This thread is full." } };
    }
    const at = new Date().toISOString();
    thread.push({ from: "dev", text: replyText, at });
    feedback.thread = thread;
    feedback.reply = replyText;
    feedback.replied_at = at;
  }
  if (archive) {
    feedback.archived = true;
  }
  const last = thread[thread.length - 1];
  const waiting = last && last.from === "user";
  const replied = thread.some((m) => m.from === "dev");
  const archived = Boolean(feedback.archived) || parsed.archived;
  await patchGist(token, id, {
    description: encodeDesc({
      kind,
      summary,
      replied,
      waiting,
      archived,
      since: parsed.since || clip(feedback.first_version, 32),
    }),
    feedback,
  });
  return {
    status: 200,
    body: { ok: true, id, replied, waiting, archived, thread },
  };
}

module.exports = async function handler(req, res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type, X-Inbox-Secret");
  if (req.method === "OPTIONS") {
    res.status(204).end();
    return;
  }
  if (!authorized(req)) {
    res.status(401).json({ error: "Inbox password required." });
    return;
  }
  const token = process.env.FEEDBACK_GITHUB_TOKEN;
  if (!token) {
    res.status(503).json({ error: "Feedback is not configured." });
    return;
  }
  try {
    const id = req.query && req.query.id;
    if (req.method === "GET" && id) {
      const gist = await readGist(token, String(id));
      if (!gist) {
        res.status(404).json({ error: "Not found." });
        return;
      }
      res.status(200).json(gist);
      return;
    }
    if (req.method === "GET") {
      const [items, downloads] = await Promise.all([listGists(token), installerDownloads(token)]);
      res.status(200).json({ items, downloads });
      return;
    }
    if (req.method === "POST") {
      const result = await applyInboxAction(token, readBody(req));
      res.status(result.status).json(result.body);
      return;
    }
    res.status(405).json({ error: "GET or POST only." });
  } catch (err) {
    res.status(502).json({ error: "Could not load the inbox.", detail: String(err.message || err).slice(0, 400) });
  }
};
