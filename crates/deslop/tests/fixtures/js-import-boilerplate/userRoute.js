import express from "express";
import { z } from "zod";
import { logger } from "../lib/logger.js";
import { db } from "../lib/db.js";
import { authenticate } from "../middleware/authenticate.js";
import { rateLimit } from "../middleware/rateLimit.js";

const router = express.Router();

router.get("/users/:id", authenticate, rateLimit, async (req, res) => {
  const user = await db.users.findById(req.params.id);
  if (!user) {
    logger.warn("user lookup miss", { id: req.params.id });
    return res.status(404).json({ error: "not found" });
  }
  return res.json({ id: user.id, name: user.name, email: user.email });
});

export default router;
