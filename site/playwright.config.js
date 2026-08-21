export default {
  testDir: "./tests",
  use: { baseURL: "http://127.0.0.1:8092" },
  webServer: {
    command: "npm run dev -- --port=8092",
    url: "http://127.0.0.1:8092/",
    reuseExistingServer: !process.env.CI,
  },
};
