// Public reactions to Deslop from named engineers — the single source of truth
// for the homepage testimonial band (partials/testimonials.njk). Quotes are
// reproduced verbatim from the linked public post and are never translated;
// only `role` and `evidence.note` are localised, keyed by page language.
// Evidence is either `pullRequests` (the figure stated by the author, one
// entry per merged pull request) or `points` (verbatim excerpts of the same
// post), so every claim on the page is checkable against the link beside it.
export default [
  {
    id: "kevmoo",
    author: "Kevin Moore",
    authorUrl: "https://kevmoo.com/",
    handle: "@kevmoo",
    role: {
      en: "Product Manager for Dart & Flutter, Google",
      zh: "Google Dart 与 Flutter 产品经理",
    },
    quote: "Holy cow dude! My initial analysis of this tool, I am blown away and super excited.",
    sourceUrl: "https://x.com/kevmoo/status/2084838485262008358",
    sourceDate: "2026-08-05",
    evidence: {
      note: {
        en: "Merged into his own Dart packages the same day:",
        zh: "当天即合并进他自己的 Dart 包：",
      },
      pullRequests: [
        { repo: "peanut.dart", url: "https://github.com/kevmoo/peanut.dart/pull/220", linesRemoved: 16 },
        { repo: "mysql.dart", url: "https://github.com/kevmoo/mysql.dart/pull/29", linesRemoved: 22 },
        { repo: "pubviz", url: "https://github.com/kevmoo/pubviz/pull/179", linesRemoved: 55 },
        { repo: "scripts.dart", url: "https://github.com/kevmoo/scripts.dart/pull/54", linesRemoved: 20 },
        { repo: "dtt", url: "https://github.com/kevmoo/dtt/pull/25", linesRemoved: 13 },
      ],
      quote: "AMAZING! SUPER useful! wow!",
      sourceUrl: "https://x.com/kevmoo/status/2084863688184578520",
    },
  },
  {
    id: "johnjkattenhorn",
    author: "John Kattenhorn",
    authorUrl: "https://x.com/johnjkattenhorn",
    handle: "@johnjkattenhorn",
    role: {
      en: "Builds companies, teams and systems",
      zh: "公司、团队与系统的构建者",
    },
    quote: "I gave Claude Code a hook it was about to write, and find-similar pointed it straight at the existing copy. That's the \"don't write a third one\" loop I want in every agent session.",
    sourceUrl: "https://x.com/johnjkattenhorn/status/2121504625878536264",
    sourceDate: "2026-11-14",
    evidence: {
      note: {
        en: "Run on his 61k-line TypeScript and Python monorepo:",
        zh: "在他 6.1 万行的 TypeScript 与 Python monorepo 上运行：",
      },
      points: [
        "341 files in about 6 seconds, ranked worst first, with HTML, JSON and text reports.",
        "One response-handling block was copied across three API routes, and one data-polling hook was copied in two views. Both fold into a single function.",
        "It found something the plain scan couldn't: two versions of the same git helper in two different reporters, written in different styles.",
      ],
    },
  },
];
