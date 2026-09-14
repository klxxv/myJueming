import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";

// Use an existing Playwright installation; this pack adds no production dependency.
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || "playwright");
const browser = await chromium.launch({ headless: true });
const output = new URL("../../test-output/companion-lab/", import.meta.url);
const assets = new URL("../../apps/desktop/src/assets/companion/puppets/v1/", import.meta.url);
await mkdir(output, { recursive: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
const errors = [];
page.on("pageerror", error => errors.push(error.message));
page.on("response", response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
await page.addInitScript(() => {
  const original = window.requestAnimationFrame.bind(window);
  window.__puppetFrameRequests = 0;
  window.requestAnimationFrame = callback => { window.__puppetFrameRequests++; return original(callback); };
});

try {
  await page.goto(process.env.COMPANION_LAB_URL || "http://127.0.0.1:1422/companion-lab.html");
  await page.locator("#rig-view").click();
  await page.locator("#puppet [data-part]").first().waitFor();
  assert.equal(await page.evaluate(() => window.__puppetFrameRequests), 0, "Static startup must not request frames");

  for (const actor of ["orange-cat", "golden-dog"]) {
    await page.locator(`[data-actor="${actor}"]`).click();
    await page.locator('[data-action="stand"]').click();
    const rig = JSON.parse(await readFile(new URL(`${actor}/rig.json`, assets), "utf8"));
    // Check every slice, including currently hidden facial replacements, for cropped geometry.
    const clipped = await page.evaluate(parts => parts.flatMap(part => {
      const group = document.querySelector(`#puppet [data-part="${part.id}"]`);
      const display = group.getAttribute("display"); group.setAttribute("display", "inline");
      const b = group.getBBox(); group.setAttribute("display", display);
      const [x, y, w, h] = part.bounds;
      return b.x < x || b.y < y || b.x + b.width > x + w || b.y + b.height > y + h ? [{ id: part.id, bbox: [b.x, b.y, b.width, b.height], bounds: part.bounds }] : [];
    }), rig.parts);
    assert.deepEqual(clipped, [], `${actor}: artwork extends outside its slice`);
    await page.screenshot({ path: new URL(`${actor}-stand.png`, output).pathname, fullPage: true });
    for (const [id, duration] of [["walk", 3600], ["jump-to-bed", 1700], ["sleep", 3800], ["go-to-garden", 5400]]) {
      await page.locator(`[data-action="${id}"]`).click();
      for (const ratio of [0, .25, .5, .75, 1]) {
        await page.locator("#timeline").fill(String(Math.round(duration * ratio)));
        const bounds = await page.locator("#puppet").evaluate(g => {
          const r = g.getBoundingClientRect(), s = document.getElementById("stage").getBoundingClientRect();
          return { inside: r.left >= s.left && r.right <= s.right && r.top >= s.top && r.bottom <= s.bottom, transform: g.getAttribute("transform") };
        });
        assert.ok(bounds.inside, `${actor}/${id}@${ratio}: actor leaves the stage`);
      }
      await page.locator("#stage").screenshot({ path: new URL(`${actor}-${id}.png`, output).pathname });
    }
    await page.locator("#separated").click();
    assert.equal(await page.locator("[data-slice]").count(), rig.parts.length);
    await page.locator('[data-slice="front-near-lower"]').click();
    await page.locator("#part-name").filter({ hasText: "下肢" }).waitFor();
    const [download] = await Promise.all([page.waitForEvent("download"), page.locator("#download-part").click()]);
    assert.equal(download.suggestedFilename(), `${actor}-front-near-lower.svg`);
    await page.screenshot({ path: new URL(`${actor}-slices.png`, output).pathname, fullPage: true });
    await page.locator("#rig-view").click();
    await page.locator("#clear-part").click();
  }

  await page.locator('[data-action="jump-to-bed"]').click();
  await page.locator("#timeline").fill("1700");
  await page.locator("#direction").selectOption("-1");
  assert.equal(await page.locator("#scene-world").getAttribute("transform"), "translate(900 0) scale(-1 1)");
  await page.locator("#show-joints").check();
  assert.ok(await page.locator("#joints circle").count() >= 26);
  await page.locator("#direction").selectOption("1");
  await page.locator("#show-joints").uncheck();

  // Verify actual finite playback, then verify the frame counter stops growing.
  await page.locator('[data-action="jump-to-bed"]').click();
  await page.locator("#speed").selectOption("1.5");
  await page.locator("#play").click();
  await page.waitForFunction(() => document.getElementById("timeline").value === "1700");
  const count = await page.evaluate(() => window.__puppetFrameRequests);
  await page.waitForTimeout(100);
  assert.equal(await page.evaluate(() => window.__puppetFrameRequests), count, "Completed action retains a frame loop");
  await page.locator('[data-action="walk"]').click();
  await page.locator("#play").click();
  await page.waitForFunction(() => Number(document.getElementById("timeline").value) > 0);
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  assert.equal(await page.locator("#play").textContent(), "播放一次");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForFunction(() => document.getElementById("play").disabled);
  await page.locator("#timeline").fill("900");
  assert.equal(await page.locator("#timeline").inputValue(), "900", "Reduced motion must preserve scrubbing");

  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.locator('[data-action="stand"]').click();
  await page.evaluate(() => document.documentElement.dataset.theme = "eye");
  await page.screenshot({ path: new URL("eye-theme.png", output).pathname, fullPage: true });
  for (const width of [900, 390]) {
    await page.setViewportSize({ width, height: 844 });
    const fits = await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth);
    assert.ok(fits, `Horizontal overflow at ${width}px`);
    await page.screenshot({ path: new URL(`responsive-${width}.png`, output).pathname, fullPage: true });
  }
  assert.deepEqual(errors, []);
  console.log("puppet lab: 60 slice bounds, both actors/four actions, downloads, finite playback, blur/reduced motion, eye theme and responsive layouts passed");
} finally {
  await browser.close();
}
