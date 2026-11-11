import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

const base = new URL(process.argv[2]);
const expected = process.argv[3];
assert.equal(base.protocol, "https:", "Pages must be published over HTTPS");
assert.match(expected ?? "", /^[0-9a-f]{40}$/, "Expected the exact deployed commit");
if (!base.pathname.endsWith("/")) base.pathname += "/";

async function resource(path) {
  const url = new URL(path, base);
  url.searchParams.set("commit", expected);
  const response = await fetch(url, {
    cache: "no-store",
    signal: AbortSignal.timeout(15_000),
  });
  assert.equal(response.status, 200, `HTTP ${response.status}: ${url}`);
  return response;
}

// Pages/CDN propagation can lag a successful deployment. Every attempt verifies
// the exact artifact identity as well as the public entry point and preview.
for (let attempt = 1; attempt <= 12; attempt++) {
  try {
    const info = await (await resource("build-info.json")).json();
    assert.equal(info.commit, expected, "Pages is still serving another commit");
    const page = await (await resource("")).text();
    assert.match(page, /briccraft/i, "The game entry point is missing");
    const metrics = await (await resource("previews/metrics.json")).json();
    assert.ok(metrics && typeof metrics === "object", "Browser evidence is missing");
    assert.equal(metrics.terrain?.parts, 12820, "Complete-area browser evidence is missing");
    assert.ok(metrics.mobile, "Mobile browser evidence is missing");
    assert.ok(metrics.gameplay?.saveRoundTrip, "Expedition persistence evidence is missing");
    assert.equal(metrics.imageQuality?.desktop?.world?.samples, 4, "Native AA evidence is missing");
    assert.equal(metrics.imageQuality?.desktop?.world?.effectWidth, 1280, "Full-density desktop evidence is missing");
    assert.equal(metrics.imageQuality?.mobile?.world?.effectWidth, 390, "Full-density phone evidence is missing");
    assert.equal(metrics.imageQuality?.frozenMenu?.newEffects, 0, "Paused-world reuse evidence is missing");
    assert.ok(metrics.designs?.saveRoundTrip, "Saved-design persistence evidence is missing");
    for (const name of ["brandenburg", "telegraph-hill", "expedition", "mobile-expedition", "mobile-terrain", "mobile-studio", "mobile-landscape", "quality-desktop", "quality-mobile", "settings", "mobile-settings", "design-library", "mobile-designs"]) {
      const preview = await resource(`previews/${name}.png`);
      assert.match(preview.headers.get("content-type") ?? "", /^image\/png\b/i);
      assert.ok((await preview.arrayBuffer()).byteLength > 1_024, `${name} preview is empty`);
    }
    console.log(`Verified ${base.href} at commit ${expected}`);
    break;
  } catch (error) {
    if (attempt === 12) throw error;
    console.log(`Waiting for Pages propagation (${attempt}/12): ${error.message}`);
    await delay(5_000);
  }
}
