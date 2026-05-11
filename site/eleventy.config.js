import techdoc from "eleventy-plugin-techdoc";
import syntaxHighlight from "@11ty/eleventy-plugin-syntaxhighlight";

const BLOG_INDEX_OVERRIDE = `---
layout: layouts/base.njk
title: Blog
permalink: /blog/
---
<div class="blog-container">
  <header class="blog-header">
    <p class="blog-eyebrow">The Manuscript · Field Notes</p>
    <h1>{{ "blog.title" | t(lang) | default("Blog") }}</h1>
    <p class="blog-subtitle">{{ "blog.subtitle" | t(lang) | default(site.description) }}</p>
  </header>

  <nav class="blog-nav">
    <a href="/blog/tags/" class="blog-nav-link">{{ "blog.tags" | t(lang) | default("Tags") }}</a>
    <a href="/blog/categories/" class="blog-nav-link">{{ "blog.categories" | t(lang) | default("Categories") }}</a>
  </nav>

  <div class="post-list">
    {%- for post in collections.posts | sort(true, false, "date") -%}
    <article class="blog-post">
      <a href="{{ post.url }}" class="post-title">{{ post.data.title }}</a>
      <div class="post-meta">
        <time datetime="{{ post.date | isoDate }}">{{ post.date | dateFormat(lang) }}</time>
        {% if post.data.author %} · {{ post.data.author }}{% endif %}
      </div>
      {% if post.data.excerpt or post.data.description %}<p class="post-excerpt">{{ post.data.excerpt | default(post.data.description) }}</p>{% endif %}
      <span class="post-more">Read <span class="material-symbols-outlined">arrow_forward</span></span>
    </article>
    {%- endfor -%}
  </div>

  {% if (collections.posts | length) == 0 %}
  <div class="blog-empty">
    <p>{{ "blog.empty" | t(lang) | default("No blog posts yet.") }}</p>
  </div>
  {% endif %}
</div>
`;

const DOCS_LAYOUT_OVERRIDE = `---
layout: layouts/base.njk
---

{% block head %}
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "TechArticle",
  "headline": "{{ title }}",
  "description": "{{ description | default(site.description) }}",
  "inLanguage": "{{ lang | default('en') }}"
}
</script>
{% endblock %}

{% set currentLang = lang | default('en') %}
{% set navPages = collections.all | eleventyNavigation %}

<div class="docs-shell">
  <aside class="docs-sidebar" id="docs-sidebar">
    <div class="docs-sidebar__brand">
      <h2>The Manuscript</h2>
      <p class="docs-sidebar__version">Live LSP + MCP · preview</p>
    </div>
    <nav class="docs-sidebar__nav" aria-label="Documentation">
      {% for entry in navPages %}
      {% set entryLang = entry.url | extractLangFromUrl(defaultLanguage) %}
      {% if entryLang == currentLang %}
      <a href="{{ entry.url }}" class="docs-sidebar__link{% if page.url == entry.url %} is-active{% endif %}">
        <span class="material-symbols-outlined">{{ entry.data.icon | default("article") }}</span>
        <span class="docs-sidebar__label">{{ entry.title }}</span>
      </a>
      {% if entry.children.length %}
      <div class="docs-sidebar__children">
        {% for child in entry.children %}
        <a href="{{ child.url }}" class="docs-sidebar__link docs-sidebar__link--child{% if page.url == child.url %} is-active{% endif %}">
          <span class="docs-sidebar__label">{{ child.title }}</span>
        </a>
        {% endfor %}
      </div>
      {% endif %}
      {% endif %}
      {% endfor %}
    </nav>
    <div class="docs-sidebar__foot">
      <a href="https://github.com/Nimblesite/Deslop" class="docs-sidebar__cta">
        <span class="material-symbols-outlined">support_agent</span>
        Community support
      </a>
    </div>
  </aside>

  <main class="docs-main">
    <article class="docs-article">
      <header class="docs-article__header">
        <div class="docs-breadcrumb">
          <a href="/docs/">Docs</a>
          <span class="material-symbols-outlined">chevron_right</span>
          <span class="docs-breadcrumb__current">{{ title | default("Introduction") }}</span>
        </div>
      </header>

      <div class="docs-article__body">
        {{ content | safe }}
      </div>

      {% if prevPage or nextPage %}
      <nav class="docs-article__footer" aria-label="Pagination">
        {% if prevPage %}
        <a href="{{ prevPage.url }}" class="docs-article__prev">
          <span class="material-symbols-outlined">arrow_back</span>
          <span>{{ prevPage.title }}</span>
        </a>
        {% else %}
        <span class="docs-article__prev is-disabled">
          <span class="material-symbols-outlined">arrow_back</span>
          <span>Previous</span>
        </span>
        {% endif %}
        {% if nextPage %}
        <a href="{{ nextPage.url }}" class="docs-article__next">
          <span>{{ nextPage.title }}</span>
          <span class="material-symbols-outlined">arrow_forward</span>
        </a>
        {% endif %}
      </nav>
      {% endif %}
    </article>
  </main>
</div>
`;

const BLOG_POST_LAYOUT_OVERRIDE = `---
layout: layouts/base.njk
---

{% block head %}
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "BlogPosting",
  "headline": "{{ title }}",
  "description": "{{ description | default(site.description) }}",
  "datePublished": "{{ date | dateToRfc3339 }}",
  "inLanguage": "{{ lang | default('en') }}"
}
</script>
{% endblock %}

{% set langPrefix = "/" + lang if lang and lang != defaultLanguage else "" %}

<article class="post-article">
  <header class="post-article__header">
    <div class="docs-breadcrumb">
      <a href="{{ langPrefix }}/blog/">Blog</a>
      <span class="material-symbols-outlined">chevron_right</span>
      <span class="docs-breadcrumb__current">{{ title }}</span>
    </div>
    <h1>{{ title }}</h1>
    <p class="post-article__meta">
      <time datetime="{{ date | isoDate }}">{{ date | dateFormat }}</time>
      {% if author %} · <span class="post-article__author">{{ author }}</span>{% endif %}
      {% if category %} · <a href="{{ langPrefix }}/blog/categories/{{ category | slugify }}/">{{ category | capitalize }}</a>{% endif %}
    </p>
    {% if tags %}
    <div class="post-article__tags">
      {% for tag in tags %}
      {% if tag != 'post' and tag != 'posts' %}
      <a href="{{ langPrefix }}/blog/tags/{{ tag | slugify }}/" class="post-article__tag">{{ tag }}</a>
      {% endif %}
      {% endfor %}
    </div>
    {% endif %}
  </header>

  <div class="post-article__body">
    {{ content | safe }}
  </div>

  <footer class="post-article__footer">
    <a href="{{ langPrefix }}/blog/" class="post-article__back">
      <span class="material-symbols-outlined">arrow_back</span>
      {{ "blog.back" | t(lang) | default("Back to Blog") }}
    </a>
  </footer>
</article>
`;

// Footer override: swap the techdoc plugin's footer markup for one that
// credits Nimblesite (the product owner) and links back to nimblesite.co.
// Techdoc ships `layouts/base.njk` as a virtual template, so we replace it
// wholesale — the only deliberate diff from upstream is the `footer-bottom`
// block. Keep the rest in lock-step with the plugin's template or head tags
// will drift.
const BASE_LAYOUT_OVERRIDE = `<!DOCTYPE html>
<html lang="{{ lang | default('en') }}">
<head>
  <script>
    (function() {
      var theme = localStorage.getItem('theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
      document.documentElement.setAttribute('data-theme', theme);
    })();
  </script>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">

  <title>{{ title | default(site.title) }}</title>
  <meta name="title" content="{{ title | default(site.title) }}">
  <meta name="description" content="{{ description | default(site.description) }}">
  {% if site.author %}<meta name="author" content="{{ site.author }}">{% endif %}
  {% if site.keywords %}<meta name="keywords" content="{{ site.keywords }}">{% endif %}
  <meta name="robots" content="index, follow">
  <meta name="generator" content="Eleventy + techdoc">
  <meta name="theme-color" content="{{ site.themeColor | default('#0066cc') }}">

  <link rel="canonical" href="{{ site.url }}{{ page.url }}">
  <link rel="icon" type="image/svg+xml" href="/assets/img/logo.svg">
  <link rel="icon" type="image/png" href="/assets/img/logo.png">
  <link rel="alternate" type="application/atom+xml" title="{{ site.title }} Feed" href="{{ site.url }}/feed.xml">

  {% for langCode in supportedLanguages %}
  <link rel="alternate" hreflang="{{ langCode }}" href="{{ site.url }}{{ page.url | altLangUrl(lang | default('en'), langCode) }}">
  {% endfor %}
  <link rel="alternate" hreflang="x-default" href="{{ site.url }}{{ page.url | altLangUrl(lang | default('en'), defaultLanguage) }}">

  <meta property="og:type" content="{% if page.url.startsWith('/blog/') and page.url != '/blog/' %}article{% else %}website{% endif %}">
  <meta property="og:url" content="{{ site.url }}{{ page.url }}">
  <meta property="og:title" content="{{ title | default(site.title) }}">
  <meta property="og:description" content="{{ description | default(site.description) }}">
  <meta property="og:site_name" content="{{ site.title }}">
  <meta property="og:locale" content="{{ (lang | default('en')) | toOgLocale }}">
  {% for langCode in supportedLanguages %}{% if langCode != (lang | default('en')) %}
  <meta property="og:locale:alternate" content="{{ langCode | toOgLocale }}">
  {% endif %}{% endfor %}
  {% if site.ogImage %}
  <meta property="og:image" content="{{ site.url }}{{ site.ogImage }}">
  <meta property="og:image:width" content="{{ site.ogImageWidth | default('1200') }}">
  <meta property="og:image:height" content="{{ site.ogImageHeight | default('630') }}">
  {% endif %}

  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:url" content="{{ site.url }}{{ page.url }}">
  <meta name="twitter:title" content="{{ title | default(site.title) }}">
  <meta name="twitter:description" content="{{ description | default(site.description) }}">
  {% if site.twitterSite %}<meta name="twitter:site" content="{{ site.twitterSite }}">{% endif %}
  {% if site.twitterCreator %}<meta name="twitter:creator" content="{{ site.twitterCreator }}">{% endif %}
  {% if site.ogImage %}<meta name="twitter:image" content="{{ site.url }}{{ site.ogImage }}">{% endif %}

  <script type="application/ld+json">
  {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "WebSite",
        "@id": "{{ site.url }}/#website",
        "url": "{{ site.url }}/",
        "name": "{{ site.title }}",
        "description": "{{ site.description }}",
        "inLanguage": "{{ lang | default('en') }}"
      },
      {
        "@type": "{% if page.url.startsWith('/docs/') %}TechArticle{% elif page.url.startsWith('/blog/') and page.url != '/blog/' %}BlogPosting{% else %}WebPage{% endif %}",
        "@id": "{{ site.url }}{{ page.url }}#webpage",
        "url": "{{ site.url }}{{ page.url }}",
        "name": "{{ title | default(site.title) }}",
        "description": "{{ description | default(site.description) }}",
        "isPartOf": { "@id": "{{ site.url }}/#website" },
        "inLanguage": "{{ lang | default('en') }}"{% if page.date %},
        "datePublished": "{{ page.date | isoDate }}"{% endif %}{% if author %},
        "author": {
          "@type": "Person",
          "name": "{{ author }}"
        }{% endif %}
      },
      {
        "@type": "BreadcrumbList",
        "@id": "{{ site.url }}{{ page.url }}#breadcrumb",
        "itemListElement": [
          {
            "@type": "ListItem",
            "position": 1,
            "name": "Home",
            "item": "{{ site.url }}/"
          }{% if page.url != '/' %},
          {
            "@type": "ListItem",
            "position": 2,
            "name": "{{ title | default('Page') }}",
            "item": "{{ site.url }}{{ page.url }}"
          }{% endif %}
        ]
      },
      {
        "@type": "Organization",
        "@id": "{{ site.url }}/#organization",
        "name": "Nimblesite",
        "url": "https://nimblesite.co"
      }
    ]
  }
  </script>

  <link rel="stylesheet" href="/techdoc/css/reset.css">
  <link rel="stylesheet" href="/techdoc/css/layout.css">
  <link rel="stylesheet" href="/techdoc/css/utilities.css">

  {% if site.stylesheet %}<link rel="stylesheet" href="{{ site.stylesheet }}">{% endif %}
  {% block head %}{% endblock %}
</head>
<body>
  <a href="#main-content" class="skip-link">Skip to main content</a>

  <header class="site-header">
    <nav class="nav">
      <div class="logo-wrap">
        <a href="{% if lang and lang != defaultLanguage %}/{{ lang }}/{% else %}/{% endif %}" class="logo">
          <img src="/assets/img/logo.svg" alt="" class="logo-mark" width="28" height="28" aria-hidden="true">
          <span class="logo-word">{{ site.name | default(site.title) }}</span>
        </a>
        <span class="logo-badge" aria-label="Live analysis server">
          <span class="logo-badge__dot" aria-hidden="true"></span>live
        </span>
      </div>

      <ul class="nav-links">
        {% set navData = navigation %}
        {% set currentLang = lang | default('en') %}
        {% for item in navData.main %}
        <li>
          {% set navUrl = item.url %}
          {% if not item.external and currentLang != defaultLanguage %}
            {% set navUrl = item.url | altLangUrl('en', currentLang) %}
          {% endif %}
          <a href="{{ navUrl }}" {% if item.external %}target="_blank" rel="noopener noreferrer"{% endif %}
             class="nav-link {% if (item.url == '/' and (page.url == '/' or page.url == '/index.html' or page.url == ('/' + currentLang + '/') or page.url == ('/' + currentLang + '/index.html'))) or (item.url != '/' and item.url | length > 1 and (page.url.startsWith(item.url) or page.url.startsWith(navUrl))) %}active{% endif %}">
            {{ item.text }}
          </a>
        </li>
        {% endfor %}
      </ul>

      <div class="nav-actions">
        {% if techdocOptions.features.darkMode %}
        <button id="theme-toggle" class="theme-toggle" aria-label="Toggle dark mode">
          <span class="theme-icon-light">☀</span>
          <span class="theme-icon-dark">☾</span>
        </button>
        {% endif %}

        <button id="mobile-menu-toggle" class="mobile-menu-toggle" aria-label="Toggle menu">
          <span></span>
          <span></span>
          <span></span>
        </button>
      </div>
    </nav>
  </header>

  <main id="main-content">
    {% block content %}{{ content | safe }}{% endblock %}
  </main>

  <footer class="site-footer">
    <div class="footer-content">
      {% if navigation.footer %}
      <div class="footer-grid">
        {% set currentLang = lang | default('en') %}
        {% for section in navigation.footer %}
        <div class="footer-section">
          <h3>{{ section.title }}</h3>
          <ul>
            {% for item in section.items %}
            {% set footerUrl = item.url %}
            {% if item.url.startsWith('/') and currentLang != defaultLanguage %}
              {% set footerUrl = item.url | altLangUrl('en', currentLang) %}
            {% endif %}
            <li><a href="{{ footerUrl }}">{{ item.text }}</a></li>
            {% endfor %}
          </ul>
        </div>
        {% endfor %}
      </div>
      {% endif %}

      <div class="footer-bottom">
        <p>&copy; {% year %} <a href="https://nimblesite.co" target="_blank" rel="noopener noreferrer">Nimblesite</a>. {{ site.name | default(site.title) }} is a Nimblesite product.</p>
      </div>
    </div>
  </footer>

  <script src="/techdoc/js/main.js" type="module"></script>

  {% block scripts %}{% endblock %}
</body>
</html>
`;

// Strip the upstream `/api/` link from llms.txt — Deslop's site doesn't
// ship an API reference route, so the placeholder would be a 404 from the
// AI-discoverability artefact itself.
const LLMS_TXT_OVERRIDE = `---json
{
  "permalink": "llms.txt",
  "eleventyExcludeFromCollections": true
}
---
# {{ site.title | default(site.name) }}

> {{ site.description }}

## Install

- VS Code extension (preferred — bundles LSP + MCP + CLI): https://github.com/Nimblesite/Deslop/releases/latest
- Homebrew (CLI only): brew install nimblesite/tap/deslop
- Scoop (CLI only): scoop install deslop

## Documentation
{% for page in collections.docs %}
- [{{ page.data.title }}]({{ site.url }}{{ page.url }}){% if page.data.description %} — {{ page.data.description }}{% endif %}
{% endfor %}

## Blog Posts
{% for post in collections.posts | reverse | limit(10) %}
- [{{ post.data.title }}]({{ site.url }}{{ post.url }}){% if post.data.excerpt or post.data.description %} — {{ post.data.excerpt | default(post.data.description) }}{% endif %}
{% endfor %}

## Navigation
- Home: {{ site.url }}/
- Documentation: {{ site.url }}/docs/
- Blog: {{ site.url }}/blog/
- Source: https://github.com/Nimblesite/Deslop
`;

const OVERRIDES = {
  "blog/index.njk": BLOG_INDEX_OVERRIDE,
  "_includes/layouts/base.njk": BASE_LAYOUT_OVERRIDE,
  "_includes/layouts/docs.njk": DOCS_LAYOUT_OVERRIDE,
  "_includes/layouts/blog.njk": BLOG_POST_LAYOUT_OVERRIDE,
  "llms.txt.njk": LLMS_TXT_OVERRIDE,
};

/**
 * Replaces plugin-registered virtual templates with project-specific ones.
 * Runs as its own plugin so `virtualTemplates` is populated by the time it
 * executes — plugin registration is queued, not immediate.
 */
function overrideVirtualTemplates(eleventyConfig) {
  const vt = eleventyConfig.virtualTemplates;
  if (!vt) return;
  for (const [key, content] of Object.entries(OVERRIDES)) {
    if (vt[key]) vt[key].content = content;
  }
}

export default function (eleventyConfig) {
  eleventyConfig.addPlugin(techdoc, {
    site: {
      name: "Deslop",
      url: "https://deslop.live",
      description:
        "The live LSP + MCP duplicate-code server for AI coding agents. Deslop streams real-time clone signals to Claude Code, Cursor, Copilot, Continue, and Codex as code is written — find-similar prevents the copy-paste before it lands. Install via the VS Code VSIX (bundles LSP, MCP server, and CLI); JetBrains plugin in active development.",
      keywords:
        "deslop, duplicate code, code clone detection, AI code duplication, LLM duplicate code, MCP server, LSP server, claude code, claude desktop, cursor, copilot, continue, codex, tree-sitter, AST clone detection, find-similar, VS Code extension, JetBrains plugin, code deduplication",
      ogImage: "/assets/img/og-card.png",
      ogImageWidth: "1200",
      ogImageHeight: "630",
    },
    features: {
      blog: true,
      docs: true,
      darkMode: true,
      i18n: false,
    },
  });

  eleventyConfig.addPlugin(syntaxHighlight);
  eleventyConfig.addPlugin(overrideVirtualTemplates);
  eleventyConfig.addPassthroughCopy("src/assets");
  eleventyConfig.addPassthroughCopy({
    "../docs/designs/logo.png": "assets/img/logo.png",
    "../docs/designs/logo.svg": "assets/img/logo.svg",
  });

  return {
    dir: { input: "src", output: "_site" },
    markdownTemplateEngine: "njk",
  };
}
