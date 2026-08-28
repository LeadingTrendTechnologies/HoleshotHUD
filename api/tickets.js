const PREFIX = "Holeshot HUD ·";
const MAX_IDS = 20;
const MAX_REPLY = 4000;
const MAX_THREAD = 40;
const ID_RE = /^[A-Za-z0-9]{8,64}$/;

function ghHeaders(token) {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "User-Agent": "Holeshot-HUD-tickets",
    "X-GitHub-Api-Version": "2022-11-28",
  };
}

function clip(s, n) {
  const t = String(s || "").replace(/\s+/g, " ").trim();
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function clipRaw(s, n) {
  const t = String(s || "");
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function isOurs(gist) {
  return String(gist.description || "").startsWith(PREFIX);
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

function parseIds(raw) {
  const parts = String(raw || "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const ids = [];
  for (const id of parts) {
    if (!ID_RE.test(id)) continue;
    if (ids.includes(id)) continue;
    ids.push(id);
    if (ids.length >= MAX_IDS) break;
  }
  return ids;
}

function parseFeedback(text) {
  try {
    const data = JSON.parse(text);
    if (data && typeof data === "object") return data;
  } catch {
    /* ignore */
  }
  return {};
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

function clipThread(thread) {
  const out = [];
  for (const item of Array.isArray(thread) ? thread : []) {
    const from = item && item.from === "dev" ? "dev" : item && item.from === "user" ? "user" : "";
    const text = clipRaw(item && item.text, MAX_REPLY).trim();
    if (!from || !text) continue;
    out.push({ from, text, at: clip(item.at, 40) || null });
    if (out.length >= MAX_THREAD) break;
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

function lastDevText(thread) {
  for (let i = thread.length - 1; i >= 0; i -= 1) {
    if (thread[i].from === "dev") return thread[i].text;
  }
  return "";
}

function ticketOf(gist) {
  if (!isOurs(gist)) return null;
  const file = gist.files && gist.files["feedback.json"];
  const data = parseFeedback(file && file.content);
  const parsed = parseDesc(gist.description);
  const thread = ensureThread(data);
  const reply = lastDevText(thread);
  const kind = data.kind === "bug" || data.kind === "feature" || data.kind === "rating" ? data.kind : parsed.kind;
  const summary = clip(data.message || parsed.summary, 72) || kind;
  return {
    id: gist.id,
    kind,
    summary,
    at: data.at || gist.created_at || null,
    reply: reply || null,
    replied_at: data.replied_at || null,
    thread,
    archived: Boolean(data.archived) || parsed.archived,
  };
}

async function readGist(token, id) {
  const gh = await fetch(`https://api.github.com/gists/${encodeURIComponent(id)}`, {
    headers: ghHeaders(token),
  });
  if (!gh.ok) return null;
  return gh.json();
}

async function patchFeedback(token, gist, feedback) {
  const parsed = parseDesc(gist.description);
  const kind = feedback.kind || parsed.kind || "other";
  const summary = parsed.summary || clip(feedback.message, 72) || kind;
  const thread = ensureThread(feedback);
  const last = thread[thread.length - 1];
  const waiting = last && last.from === "user";
  const replied = thread.some((m) => m.from === "dev");
  const archived = Boolean(feedback.archived) || parsed.archived;
  const gh = await fetch(`https://api.github.com/gists/${encodeURIComponent(gist.id)}`, {
    method: "PATCH",
    headers: ghHeaders(token),
    body: JSON.stringify({
      description: encodeDesc({
        kind,
        summary,
        replied,
        waiting,
        archived,
        since: parsed.since || clip(feedback.first_version, 32),
      }),
      files: { "feedback.json": { content: JSON.stringify(feedback, null, 2) } },
    }),
  });
  if (!gh.ok) {
    const err = await gh.text();
    throw new Error(err.slice(0, 400));
  }
}

async function addUserMessage(token, id, message) {
  const gist = await readGist(token, id);
  if (!gist || !isOurs(gist)) return { status: 404, body: { error: "Not found." } };
  const file = gist.files && gist.files["feedback.json"];
  const feedback = parseFeedback(file && file.content);
  const thread = ensureThread(feedback);
  if (thread.length >= MAX_THREAD) {
    return { status: 400, body: { error: "This thread is full." } };
  }
  thread.push({ from: "user", text: message, at: new Date().toISOString() });
  feedback.thread = thread;
  await patchFeedback(token, gist, feedback);
  return { status: 200, body: { ok: true, id, thread } };
}

module.exports = async function handler(req, res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");
  if (req.method === "OPTIONS") {
    res.status(204).end();
    return;
  }

  const token = process.env.FEEDBACK_GITHUB_TOKEN;
  if (!token) {
    res.status(503).json({ error: "Feedback is not configured." });
    return;
  }

  if (req.method === "POST") {
    const data = readBody(req);
    const id = String(data.id || "").trim();
    const message = clipRaw(data.message, MAX_REPLY).trim();
    if (!ID_RE.test(id)) {
      res.status(400).json({ error: "Missing ticket id." });
      return;
    }
    if (!message) {
      res.status(400).json({ error: "Write a reply first." });
      return;
    }
    try {
      const result = await addUserMessage(token, id, message);
      res.status(result.status).json(result.body);
    } catch (err) {
      res.status(502).json({
        error: "Could not send the reply.",
        detail: String(err.message || err).slice(0, 400),
      });
    }
    return;
  }

  if (req.method !== "GET") {
    res.status(405).json({ error: "GET or POST only" });
    return;
  }

  const ids = parseIds(req.query && req.query.ids);
  if (!ids.length) {
    res.status(200).json({ tickets: [] });
    return;
  }

  try {
    const gists = await Promise.all(ids.map((id) => readGist(token, id)));
    const tickets = [];
    for (const gist of gists) {
      if (!gist) continue;
      const ticket = ticketOf(gist);
      if (ticket) tickets.push(ticket);
    }
    res.status(200).json({ tickets });
  } catch (err) {
    res.status(502).json({
      error: "Could not load tickets.",
      detail: String(err.message || err).slice(0, 400),
    });
  }
};

module.exports.config = {
  api: { bodyParser: { sizeLimit: "32kb" } },
};
