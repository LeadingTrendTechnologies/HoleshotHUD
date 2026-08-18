const REPO = "LeadingTrendTechnologies/HoleshotHUD";
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
  const t = String(s || "");
  return t.length <= n ? t : `${t.slice(0, n)}…`;
}

function gistName(name) {
  const base = String(name || "last-race.jsonl").replace(/[^A-Za-z0-9._-]/g, "_");
  return base.toLowerCase().endsWith(".jsonl") ? base : `${base}.jsonl`;
}

async function createGist(token, name, content) {
  if (!content || !String(content).trim()) return null;
  const gh = await fetch("https://api.github.com/gists", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "User-Agent": "Holeshot-HUD-feedback",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: JSON.stringify({
      description: "Holeshot HUD last race log",
      public: false,
      files: { [gistName(name)]: { content: String(content) } },
    }),
  });
  if (!gh.ok) return null;
  const created = await gh.json();
  return created.html_url || null;
}

async function uploadLogFile(token, name, content) {
  if (!content || !String(content).trim()) return null;
  const path = `feedback-logs/${Date.now()}-${gistName(name)}`;
  const gh = await fetch(`https://api.github.com/repos/${REPO}/contents/${encodeURIComponent(path).replace(/%2F/g, "/")}`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "User-Agent": "Holeshot-HUD-feedback",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: JSON.stringify({
      message: `Add last race log ${gistName(name)}`,
      content: Buffer.from(String(content), "utf8").toString("base64"),
    }),
  });
  if (!gh.ok) return null;
  const created = await gh.json();
  return created.content?.html_url || null;
}

function issueBody(data, logUrl) {
  const lines = [
    `**Holeshot HUD ${clip(data.version, 32)}**`,
    `${clip(data.os, 64)}`,
    "",
  ];
  if (data.rating) lines.push(`Rating: ${data.rating}/5`, "");
  if (data.track) lines.push(`Track: ${clip(data.track, 80)}`, "");
  if (data.message) lines.push(clip(data.message, 4000), "");
  const fileName = clip(data.log_name || "last-race.jsonl", 80);
  if (logUrl) {
    lines.push("", `Last race log: [${fileName}](${logUrl})`);
    if (data.log_truncated) lines.push("", "_Log was truncated to the last 400 KB._");
  } else if (data.log && data.log.includes('"cur":')) {
    lines.push("", `<details><summary>${fileName}</summary>`, "", "```jsonl", clip(data.log, 50000), "```", "</details>");
  } else {
    lines.push("", "_No race log attached._");
  }
  return lines.join("\n");
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
  const kind = data.kind === "bug" ? "bug" : "rating";
  const message = clip(data.message, 4000).trim();
  const rating = Number(data.rating) || 0;
  if (kind === "bug" && !message) {
    res.status(400).json({ error: "Describe the bug first." });
    return;
  }
  if (kind === "rating" && (rating < 1 || rating > 5) && !message) {
    res.status(400).json({ error: "Pick a star rating first." });
    return;
  }

  const raw = typeof req.body === "string" ? req.body : JSON.stringify(data);
  if (raw.length > MAX_BODY) {
    res.status(413).json({ error: "Feedback is too large." });
    return;
  }

  const gistUrl = data.log ? await createGist(token, data.log_name, data.log) : null;
  const logUrl = gistUrl || (data.log ? await uploadLogFile(token, data.log_name, data.log) : null);

  const title =
    kind === "rating"
      ? `Rating: ${rating || "?"}/5`
      : `Bug: ${clip(message.replace(/\s+/g, " "), 72) || "report"}`;

  const gh = await fetch(`https://api.github.com/repos/${REPO}/issues`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "User-Agent": "Holeshot-HUD-feedback",
      "X-GitHub-Api-Version": "2022-11-28",
    },
    body: JSON.stringify({ title, body: issueBody({ ...data, rating, message }, logUrl) }),
  });
  if (!gh.ok) {
    const err = await gh.text();
    res.status(502).json({ error: "Could not create the issue.", detail: clip(err, 400) });
    return;
  }
  const created = await gh.json();
  res.status(201).json({ ok: true, url: created.html_url || null, log: logUrl || null });
};

module.exports.config = {
  api: { bodyParser: { sizeLimit: "2mb" } },
};
