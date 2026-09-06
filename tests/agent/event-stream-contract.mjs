import assert from "node:assert/strict";

/**
 * Reconciles a v1 AppEvent with a projection watermark.
 *
 * This mirrors the public subscribe-before-projection contract without
 * importing renderer code. A future UI integration test can import this
 * helper and compare its behaviour to `agentClient.subscribe`.
 */
export function reconcileEvent({ snapshotSequence, lastSequence, event }) {
  assert.match(String(snapshotSequence), /^\d+$/, "snapshot sequence must be a decimal string");
  assert.match(String(lastSequence), /^\d+$/, "last sequence must be a decimal string");
  assert.equal(event.contract_version, "1.0", "event contract version must be 1.0");
  assert.match(String(event.sequence), /^\d+$/, "event sequence must be a decimal string");

  const sequence = BigInt(event.sequence);
  const floor = BigInt(lastSequence);
  if (sequence <= floor) return { action: "discard", nextSequence: String(floor) };
  if (sequence !== floor + 1n) return { action: "resnapshot", nextSequence: String(floor) };
  return { action: "apply", nextSequence: String(sequence) };
}

function event(sequence) {
  return {
    contract_version: "1.0",
    sequence,
    kind: "context_changed",
    binding_id: "018f7b1a-2c40-7e33-9a11-1e3a98d0f021",
    origin: "native",
    payload: {},
  };
}

assert.deepEqual(reconcileEvent({ snapshotSequence: "41", lastSequence: "41", event: event("41") }), {
  action: "discard",
  nextSequence: "41",
});
assert.deepEqual(reconcileEvent({ snapshotSequence: "41", lastSequence: "41", event: event("42") }), {
  action: "apply",
  nextSequence: "42",
});
assert.deepEqual(reconcileEvent({ snapshotSequence: "41", lastSequence: "41", event: event("43") }), {
  action: "resnapshot",
  nextSequence: "41",
});
assert.deepEqual(
  reconcileEvent({ snapshotSequence: "9007199254740992", lastSequence: "9007199254740992", event: event("9007199254740993") }),
  { action: "apply", nextSequence: "9007199254740993" },
);

console.log("event-stream contract: passed");
