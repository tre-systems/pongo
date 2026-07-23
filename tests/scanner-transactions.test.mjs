import assert from "node:assert/strict";
import test from "node:test";
import { isScannerTransaction } from "../worker/scanner-transactions.mjs";

test("identifies known exploit-probe transactions", () => {
  for (const transaction of [
    "GET /ms-edit.php",
    "GET /*",
    "GET /_ignition/execute-solution",
    "GET /wp-includes/blocks/audio/",
    "HEAD //edit.php",
    "/ccc.php",
    "//a1.php",
  ]) {
    assert.equal(isScannerTransaction(transaction), true, transaction);
  }
});

test("keeps product transactions", () => {
  for (const transaction of ["GET /", "GET /create", "GET /join/ABC123", "GET /ws/ABC123"]) {
    assert.equal(isScannerTransaction(transaction), false, transaction);
  }
});
