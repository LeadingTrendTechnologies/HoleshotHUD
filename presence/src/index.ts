import { DurableObject } from "cloudflare:workers";

const CORS: Record<string, string> = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
};

const STALE_MS = 45 * 60 * 1000;
const CAP = 40;

type Env = {
  PRESENCE: DurableObjectNamespace;
};

type Body = {
  session?: unknown;
  client_id?: unknown;
  race_num?: unknown;
  name?: unknown;
  leave?: unknown;
};

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json", ...CORS },
  });
}

export class PresenceRoom extends DurableObject<Env> {
  async fetch(request: Request): Promise<Response> {
    const body = (await request.json().catch(() => ({}))) as Body;
    const clientId = String(body.client_id || "").slice(0, 64);
    if (!clientId) {
      return json({ error: "client_id required" }, 400);
    }

    const sql = this.ctx.storage.sql;
    sql.exec(
      `CREATE TABLE IF NOT EXISTS riders (
        client_id TEXT PRIMARY KEY,
        race_num INTEGER NOT NULL,
        name TEXT NOT NULL,
        at INTEGER NOT NULL
      )`,
    );
    const now = Date.now();
    sql.exec(`DELETE FROM riders WHERE at < ?`, now - STALE_MS);

    if (body.leave) {
      sql.exec(`DELETE FROM riders WHERE client_id = ?`, clientId);
    } else {
      const raceNum = Number(body.race_num) || 0;
      const name = String(body.name || "").slice(0, 64);
      const mine = sql.exec(`SELECT client_id FROM riders WHERE client_id = ?`, clientId).toArray();
      const n = Number(sql.exec(`SELECT COUNT(*) AS c FROM riders`).one().c);
      if (mine.length > 0 || n < CAP) {
        sql.exec(
          `INSERT INTO riders (client_id, race_num, name, at) VALUES (?, ?, ?, ?)
           ON CONFLICT(client_id) DO UPDATE SET race_num = excluded.race_num, name = excluded.name, at = excluded.at`,
          clientId,
          raceNum,
          name,
          now,
        );
      }
    }

    const riders = sql
      .exec(`SELECT race_num, name FROM riders`)
      .toArray()
      .map((row) => ({
        race_num: Number(row.race_num) || 0,
        name: String(row.name || ""),
      }));
    return json({ riders });
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: CORS });
    }
    if (request.method !== "POST") {
      return json({ error: "POST only" }, 405);
    }
    const body = (await request.json().catch(() => ({}))) as Body;
    const session = String(body.session || "").trim().slice(0, 120);
    const clientId = String(body.client_id || "").trim().slice(0, 64);
    if (!session || !clientId) {
      return json({ error: "session and client_id required" }, 400);
    }
    const id = env.PRESENCE.idFromName(session);
    const stub = env.PRESENCE.get(id);
    return stub.fetch(
      new Request(request.url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }),
    );
  },
} satisfies ExportedHandler<Env>;
