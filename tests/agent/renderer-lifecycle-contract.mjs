import assert from "node:assert/strict";

/** A renderer adapter must return one disposer for every acquired resource. */
export function assertRendererLifecycle(createRenderer) {
  const calls = [];
  const renderer = createRenderer({
    addEventListener: (event) => calls.push(`listen:${event}`),
    removeEventListener: (event) => calls.push(`unlisten:${event}`),
    requestAnimationFrame: () => { calls.push("frame"); return 7; },
    cancelAnimationFrame: (id) => calls.push(`cancel:${id}`),
    disposeGpu: () => calls.push("dispose-gpu"),
  });
  assert.equal(typeof renderer.dispose, "function", "renderer must expose dispose()");
  renderer.dispose();
  assert.ok(calls.includes("cancel:7"), "dispose must cancel its animation frame");
  assert.ok(calls.includes("dispose-gpu"), "dispose must release GPU resources");
  assert.ok(calls.some((call) => call.startsWith("unlisten:")), "dispose must remove event listeners");
}

assertRendererLifecycle((platform) => {
  platform.addEventListener("visibilitychange");
  platform.requestAnimationFrame(() => {});
  return {
    dispose() {
      platform.cancelAnimationFrame(7);
      platform.removeEventListener("visibilitychange");
      platform.disposeGpu();
    },
  };
});
console.log("renderer lifecycle contract: passed");
