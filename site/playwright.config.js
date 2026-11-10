const testPort = process.env.DESLOP_SITE_TEST_PORT || "8092";
const publicUrl = process.env.DESLOP_SITE_TEST_URL;
const testUrl = publicUrl || `http://127.0.0.1:${testPort}`;

export default {
  testDir: "./tests",
  use: { baseURL: testUrl },
  // [DOC-FILMS-DELIVERY] Safari's engine is the one that froze on playback-speed changes.
  projects: [
    { name: "chromium", use: { browserName: "chromium" }, testIgnore: /playback-speed/ },
    { name: "webkit", use: { browserName: "webkit" }, testMatch: /playback-speed/ },
  ],
  webServer: publicUrl ? undefined : {
    command: `npm run dev -- --port=${testPort}`,
    url: `${testUrl}/`,
    reuseExistingServer: !process.env.CI,
  },
};
