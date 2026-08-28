const PREFIX = "Holeshot HUD ·";
const MAX_BODY = 1_500_000;

function readBody(req) {
  if (req.body == null) return {};
  if (typeof req.body === "object") return req.body;
  try {
    return JSON.parse(req.body);
  } catch {
    return {};
  }
}

function clip(s, n) {
  const t = String(s || "").replace(/\s+/g, " ").trim();
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function clipRaw(s, n) {
  const t = String(s || "");
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function clipLogTail(s, n) {
  const t = String(s || "");
  if (t.length <= n) return { log: t, truncated: false };
  const start = t.length - n;
  const nl = t.indexOf("\n", start);
  return { log: t.slice(nl >= 0 ? nl + 1 : start), truncated: true };
}

function kindOf(data) {
  if (data.kind === "bug") return "bug";
  if (data.kind === "feature") return "feature";
  return "rating";
}

function summary(kind, data) {
  if (kind === "rating") return `${data.rating || "?"}/5`;
  return clip(data.message, 72) || kind;
}

function sinceTag(data) {
  const v = clip(data.first_version, 32);
  return v ? `[since ${v}] ` : "";
}

async function saveGist(token, kind, data) {
  const files = {
    "feedback.json": {
      content: JSON.stringify(
        {
          kind,
          rating: data.rating || 0,
          message: clipRaw(data.message, 4000),
          version: clip(data.version, 32),
          first_version: clip(data.first_version, 32),
          os: clip(data.os, 64),
          track: clip(data.track, 80),
          log_name: data.log ? clip(data.log_name || "race.jsonl", 80) : null,
          log_truncated: Boolean(data.log_truncated),
          log_skipped: Boolean(data.log_skipped),
          at: new Date().toISOString(),
        },
        null,
        2
      ),
    },
  };
  if (typeof data.log === "string" && data.log.trim()) {
    const name = clip(data.log_name || "race.jsonl", 80).replace(/[^A-Za-z0-9._-]/g, "_");
    files[name.endsWith(".jsonl") ? name : "race.jsonl"] = { content: data.log };
  }
  const gh = await fetch("https://api.github.com/gists", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "User-Agent": "Holeshot-HUD-feedback",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: JSON.stringify({
      description: `${PREFIX} ${kind} · ${sinceTag(data)}${summary(kind, data)}`,
      public: false,
      files,
    }),
  });
  if (!gh.ok) {
    const err = await gh.text();
    return { error: err };
  }
  const created = await gh.json();
  return { id: created.id, url: created.html_url || null };
}

module.exports = async function handler(req, res) {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "POST, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");
  if (req.method === "OPTIONS") {
    res.status(204).end();
    return;
  }
  if (req.method !== "POST") {
    res.status(405).json({ error: "POST only" });
    return;
  }

  const token = process.env.FEEDBACK_GITHUB_TOKEN;
  if (!token) {
    res.status(503).json({ error: "Feedback is not configured." });
    return;
  }

  const data = readBody(req);
  if (typeof data.log === "string" && data.log) {
    const clipped = clipLogTail(data.log, 700_000);
    data.log = clipped.log;
    if (clipped.truncated) data.log_truncated = true;
  }
  const kind = kindOf(data);
  const message = clipRaw(data.message, 4000).trim();
  const rating = Number(data.rating) || 0;
  if ((kind === "bug" || kind === "feature") && !message) {
    res.status(400).json({ error: kind === "feature" ? "Describe the feature first." : "Describe the bug first." });
    return;
  }
  if (kind === "rating" && (rating < 1 || rating > 5) && !message) {
    res.status(400).json({ error: "Pick a star rating first." });
    return;
  }

  const raw = JSON.stringify(data);
  if (raw.length > MAX_BODY) {
    res.status(413).json({ error: "Feedback is too large." });
    return;
  }

  const saved = await saveGist(token, kind, { ...data, rating, message });
  if (saved.error) {
    res.status(502).json({
      error: "Could not save feedback. Token needs gist access.",
      detail: clip(saved.error, 400),
    });
    return;
  }
  res.status(201).json({ ok: true, id: saved.id || null });
};

module.exports.config = {
  api: { bodyParser: { sizeLimit: "2mb" } },
};
