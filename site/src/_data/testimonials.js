// Public reactions to Deslop from named engineers — the single source of truth
// for the homepage testimonial band (partials/testimonials.njk). Quotes are
// reproduced verbatim from the linked public post and are never translated;
// only `role` and `evidence.note` are localised, keyed by page language.
// `linesRemoved` is the figure stated by the author in that same post, one
// entry per merged pull request, so every number on the page is checkable
// against the link beside it.
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
];
