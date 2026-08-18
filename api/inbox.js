const PREFIX = "Holeshot HUD ·";

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

function parseDesc(description) {
  const rest = String(description || "").replace(PREFIX, "").trim();
  const sep = rest.indexOf("·");
  if (sep < 0) return { kind: "other", summary: rest };
  return { kind: rest.slice(0, sep).trim(), summary: rest.slice(sep + 1).trim() };
}

function isOurs(gist) {
  return String(gist.description || "").startsWith(PREFIX);
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
      const { kind, summary } = parseDesc(gist.description);
      items.push({
        id: gist.id,
        kind,
        summary,
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
  return {
    id: gist.id,
    description: gist.description,
    at: gist.created_at,
    url: gist.html_url,
    files,
  };
}

async function deleteGist(token, id) {
  const current = await readGist(token, id);
  if (!current) return false;
  const gh = await fetch(`https://api.github.com/gists/${encodeURIComponent(id)}`, {
    method: "DELETE",
    headers: ghHeaders(token),
  });
  return gh.ok || gh.status === 204;
}

module.exports = async function handler(req, res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, DELETE, OPTIONS");
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
      res.status(200).json({ items: await listGists(token) });
      return;
    }
    if (req.method === "DELETE" && id) {
      const ok = await deleteGist(token, String(id));
      res.status(ok ? 204 : 404).end();
      return;
    }
    res.status(405).json({ error: "GET or DELETE only." });
  } catch (err) {
    res.status(502).json({ error: "Could not load the inbox.", detail: String(err.message || err).slice(0, 400) });
  }
};
