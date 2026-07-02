import express from "express";
import { z } from "zod";
import { logger } from "../lib/logger.js";
import { db } from "../lib/db.js";
import { authenticate } from "../middleware/authenticate.js";
import { rateLimit } from "../middleware/rateLimit.js";

const router = express.Router();

const placement = z.object({ sku: z.string(), quantity: z.number().int().positive() });

router.post("/orders", authenticate, rateLimit, async (req, res) => {
  const parsed = placement.safeParse(req.body);
  if (!parsed.success) {
    return res.status(422).json({ errors: parsed.error.flatten() });
  }
  const order = await db.orders.create(parsed.data);
  logger.info("order placed", { orderId: order.id });
  return res.status(201).json(order);
});

export default router;
