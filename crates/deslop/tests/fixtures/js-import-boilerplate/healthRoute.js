import express from "express";
import { z } from "zod";
import { logger } from "../lib/logger.js";
import { db } from "../lib/db.js";
import { authenticate } from "../middleware/authenticate.js";
import { rateLimit } from "../middleware/rateLimit.js";

const router = express.Router();

router.get("/health", async (_req, res) => {
  const reachable = await db.ping();
  logger.debug("health probe", { reachable });
  return res.json({ status: reachable ? "ok" : "degraded", uptime: process.uptime() });
});

export default router;
