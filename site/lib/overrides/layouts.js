// Per-language blog-index front matter. `title`/`description` feed <title>
// and the meta description (Eleventy does not template front-matter values);
// the visible H1, subtitle, eyebrow, and "read more" come from i18n.json so
// every string has one source of truth.
const BLOG_INDEX_META = {
  en: {
    title: "Duplicate Code & AI Coding Agents Blog | Deslop",
    description:
      "Field notes on duplicate-code detection for AI coding agents — ranking, tree-sitter parsing, the MCP and LSP servers, and why prevention beats cleanup.",
  },
  zh: {
    title: "重复代码与 AI 编码智能体博客 | Deslop",
    description:
      "面向 AI 编码智能体的重复代码检测现场札记——排名机制、tree-sitter 解析、MCP 与 LSP 服务器，以及为何预防胜于清理。",
  },
};

/**
 * Builds the blog-index virtual template for one language. The default
 * language renders at /blog/ from `collections.posts`; every other language
 * renders at /<lang>/blog/ from `collections.<lang>posts`. One body, no
 * per-language copies.
 */
export const blogIndexOverride = (lang) => {
  const isDefault = lang === "en";
  const meta = BLOG_INDEX_META[lang];
  const posts = isDefault ? "collections.posts" : `collections.${lang}posts`;
  const frontmatter = isDefault
    ? `permalink: /blog/`
    : `permalink: /${lang}/blog/\nlang: ${lang}`;
  return `---
layout: layouts/base.njk
title: ${meta.title}
description: ${meta.description}
${frontmatter}
---
<div class="blog-container">
  <header class="blog-header">
    <p class="blog-eyebrow">{{ "blog.eyebrow" | t(lang) | default("Field notes") }}</p>
    <h1>{{ "blog.indexTitle" | t(lang) | default("Duplicate-code engineering") }}</h1>
    <p class="blog-subtitle">{{ "blog.subtitle" | t(lang) | default(site.description) }}</p>
  </header>

  <div class="post-grid">
    {%- for post in ${posts} | sort(true, false, "date") -%}
    <article class="post-card{% if loop.first %} post-card--featured{% endif %}">
      <a href="{{ post.url }}" class="post-card__thumb" tabindex="-1" aria-hidden="true">
        <img src="{{ post.data.heroImage | default(site.ogImage) }}"
             srcset="{{ post.data.heroImage | replace('.webp', '-800.webp') }} 800w, {{ post.data.heroImage }} 1600w"
             sizes="(max-width: 40rem) calc(100vw - 3rem), (max-width: 64rem) 50vw, 42rem"
             alt="" width="{{ post.data.heroImageWidth | default('1200') }}" height="{{ post.data.heroImageHeight | default('630') }}"
             loading="lazy" decoding="async">
      </a>
      <div class="post-card__body">
        <div class="post-card__meta">
          <time datetime="{{ post.date | isoDate }}">{{ post.date | dateFormat(lang) }}</time>
          {% if post.data.author %}<span class="post-card__sep">·</span>{{ post.data.author }}{% endif %}
        </div>
        <a href="{{ post.url }}" class="post-card__title">{{ post.data.title }}</a>
        {% if post.data.excerpt or post.data.description %}<p class="post-card__excerpt">{{ post.data.excerpt | default(post.data.description) }}</p>{% endif %}
        <a href="{{ post.url }}" class="post-card__more">{{ "blog.readMore" | t(lang) | default("Read Article") }} <span class="material-symbols-outlined">arrow_forward</span></a>
      </div>
    </article>
    {%- endfor -%}
  </div>

  {% if (${posts} | length) == 0 %}
  <div class="blog-empty">
    <p>{{ "blog.empty" | t(lang) | default("No blog posts yet.") }}</p>
  </div>
  {% endif %}
</div>
`;
};

export const DOCS_LAYOUT_OVERRIDE = `---
layout: layouts/base.njk
---

{% set currentLang = lang | default('en') %}
{% set langPrefix = "/" + currentLang if currentLang != defaultLanguage else "" %}
{% set navPages = collections.all | eleventyNavigation %}

<div class="docs-shell">
  {% include "partials/docs-sidebar.njk" %}

  <main class="docs-main">
    <article class="docs-article">
      {% include "partials/prose-hero.njk" %}
      <header class="docs-article__header">
        <div class="docs-breadcrumb">
          <a href="{{ langPrefix }}/docs/">{{ "nav.docs" | t(currentLang) | default("Docs") }}</a>
          <span class="material-symbols-outlined">chevron_right</span>
          <span class="docs-breadcrumb__current">{{ eleventyNavigation.title | default(eleventyNavigation.key) | default(title) | default("Introduction") }}</span>
        </div>
      </header>

      <div class="prose prose--docs">
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
          <span>{{ "docs.previous" | t(currentLang) | default("Previous") }}</span>
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

export const BLOG_POST_LAYOUT_OVERRIDE = `---
layout: layouts/base.njk
---

{% set langPrefix = "/" + lang if lang and lang != defaultLanguage else "" %}

<article class="post-article">
  {% include "partials/prose-hero.njk" %}
  <header class="post-article__header">
    <div class="docs-breadcrumb">
      <a href="{{ langPrefix }}/blog/">{{ "blog.title" | t(lang) | default("Blog") }}</a>
      <span class="material-symbols-outlined">chevron_right</span>
      <span class="docs-breadcrumb__current">{{ title }}</span>
    </div>
    <h1>{{ title }}</h1>
    <p class="post-article__meta">
      <time datetime="{{ date | isoDate }}">{{ date | dateFormat(lang) }}</time>
      {% if author %} · <span class="post-article__author">{{ author }}</span>{% endif %}
    </p>
  </header>

  <div class="prose">
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

// Project-owned shell. Techdoc ships this as a virtual template, so metadata,
// localization, structured data, analytics, and the Nimblesite credit live in
// one replacement instead of drifting across page layouts.
export const BASE_LAYOUT_OVERRIDE = `<!DOCTYPE html>
{#- Locale-safe i18n: derive the effective language and a locale-stripped base
    path straight from the URL, so language alternates never double-prefix
    (/zh/zh/...) even when an auto-generated page reports the wrong lang. Set
    noTranslation: true in a page's front matter to opt it out of the language
    cluster entirely. -#}
{%- set effLang = 'zh' if (page.url == '/zh/' or page.url.startsWith('/zh/')) else (lang | default('en')) -%}
{%- set basePath = (page.url | replace('/zh/', '/')) if effLang == 'zh' else page.url -%}
{%- set isHome = basePath == '/' -%}
{%- set isDocsPage = basePath.startsWith('/docs/') or docsShell -%}
{%- set isBlogPost = basePath.startsWith('/blog/') and basePath != '/blog/' -%}
{%- set canonicalUrl = site.url + page.url -%}
{%- set websiteId = site.url + '/#website' -%}
{%- set organizationId = site.url + '/#organization' -%}
{%- set webpageId = canonicalUrl + '#webpage' -%}
{%- set breadcrumbId = canonicalUrl + '#breadcrumb' -%}
{%- set currentTitle = title | default(site.title) -%}
{%- set currentDescription = description | default(site.description) -%}
{%- set homeUrl = site.url + ('/zh/' if effLang == 'zh' else '/') -%}
{%- set homeName = 'nav.home' | t(effLang) | default('Home') -%}
{%- set sectionName = '' -%}
{%- set sectionUrl = '' -%}
{%- if isDocsPage -%}
  {%- set sectionName = 'nav.docs' | t(effLang) | default('Docs') -%}
  {%- set sectionUrl = site.url + ('/zh/docs/' if effLang == 'zh' else '/docs/') -%}
{%- elif basePath.startsWith('/blog/') -%}
  {%- set sectionName = 'blog.title' | t(effLang) | default('Blog') -%}
  {%- set sectionUrl = site.url + ('/zh/blog/' if effLang == 'zh' else '/blog/') -%}
{%- endif -%}
<html lang="{{ effLang }}">
<head>
  <script>
    (function() {
      var theme = localStorage.getItem('theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
      document.documentElement.setAttribute('data-theme', theme);
    })();
  </script>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">

  <!-- Google tag (gtag.js) -->
  <script>
    window.dataLayer = window.dataLayer || [];
    function gtag(){dataLayer.push(arguments);}
    gtag('js', new Date());

    gtag('config', 'G-F8YJ86ETVQ');
    // [SITE-LOAD-ANALYTICS] Analytics must not delay the page's load event.
    window.addEventListener('load', function() {
      var analytics = document.createElement('script');
      analytics.async = true;
      analytics.src = 'https://www.googletagmanager.com/gtag/js?id=G-F8YJ86ETVQ';
      document.head.appendChild(analytics);
    }, { once: true });
  </script>

  <title>{{ title | default(site.title) }}</title>
  <meta name="title" content="{{ title | default(site.title) }}">
  <meta name="description" content="{{ description | default(site.description) }}">
  {% if site.author %}<meta name="author" content="{{ site.author }}">{% endif %}
  <meta name="robots" content="index, follow">
  <meta name="generator" content="Eleventy + techdoc">
  <meta name="theme-color" content="{{ site.themeColor | default('#0066cc') }}">
  {% set pageOgImage = ogImage | default(site.ogImage) %}
  {% set pageOgImageWidth = ogImageWidth | default(site.ogImageWidth) | default('1200') %}
  {% set pageOgImageHeight = ogImageHeight | default(site.ogImageHeight) | default('630') %}

  <link rel="canonical" href="{{ site.url }}{{ page.url }}">
  <link rel="icon" type="image/svg+xml" href="/assets/img/logo.svg">
  <link rel="icon" type="image/png" href="/assets/img/logo.png">
  <link rel="alternate" type="application/atom+xml" title="{{ site.title }} Feed" href="{{ site.url }}/feed.xml">

  {%- if noTranslation %}
  <link rel="alternate" hreflang="{{ effLang }}" href="{{ site.url }}{{ page.url }}">
  <link rel="alternate" hreflang="x-default" href="{{ site.url }}{{ page.url }}">
  {%- else %}
  {% for langCode in supportedLanguages %}
  <link rel="alternate" hreflang="{{ langCode }}" href="{{ site.url }}{% if langCode == defaultLanguage %}{{ basePath }}{% else %}/{{ langCode }}{{ basePath }}{% endif %}">
  {% endfor %}
  <link rel="alternate" hreflang="x-default" href="{{ site.url }}{{ basePath }}">
  {%- endif %}

  <meta property="og:type" content="{% if isBlogPost %}article{% else %}website{% endif %}">
  <meta property="og:url" content="{{ site.url }}{{ page.url }}">
  <meta property="og:title" content="{{ title | default(site.title) }}">
  <meta property="og:description" content="{{ description | default(site.description) }}">
  <meta property="og:site_name" content="{{ site.title }}">
  <meta property="og:locale" content="{{ effLang | toOgLocale }}">
  {%- if not noTranslation %}
  {% for langCode in supportedLanguages %}{% if langCode != effLang %}
  <meta property="og:locale:alternate" content="{{ langCode | toOgLocale }}">
  {% endif %}{% endfor %}
  {%- endif %}
  {% if pageOgImage %}
  <meta property="og:image" content="{{ site.url }}{{ pageOgImage }}">
  <meta property="og:image:secure_url" content="{{ site.url }}{{ pageOgImage }}">
  <meta property="og:image:type" content="{{ pageOgImage | toImageMimeType }}">
  <meta property="og:image:width" content="{{ pageOgImageWidth }}">
  <meta property="og:image:height" content="{{ pageOgImageHeight }}">
  <meta property="og:image:alt" content="{{ ogImageAlt | default('social.defaultImageAlt' | t(effLang)) | default(site.title) }}">
  {% endif %}

  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:url" content="{{ site.url }}{{ page.url }}">
  <meta name="twitter:title" content="{{ title | default(site.title) }}">
  <meta name="twitter:description" content="{{ description | default(site.description) }}">
  {% if site.twitterSite %}<meta name="twitter:site" content="{{ site.twitterSite }}">{% endif %}
  {% if site.twitterCreator %}<meta name="twitter:creator" content="{{ site.twitterCreator }}">{% endif %}
  {% if pageOgImage %}<meta name="twitter:image" content="{{ site.url }}{{ pageOgImage }}">
  <meta name="twitter:image:alt" content="{{ ogImageAlt | default('social.defaultImageAlt' | t(effLang)) | default(site.title) }}">{% endif %}

  <script type="application/ld+json">
  {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "WebSite",
        "@id": {{ websiteId | jsonValue | safe }},
        "url": {{ (site.url + '/') | jsonValue | safe }},
        "name": {{ site.title | jsonValue | safe }},
        "description": {{ site.description | jsonValue | safe }},
        "inLanguage": ["en", "zh"],
        "publisher": { "@id": {{ organizationId | jsonValue | safe }} }
      },
      {
        "@type": "{% if isDocsPage %}TechArticle{% elif isBlogPost %}BlogPosting{% else %}WebPage{% endif %}",
        "@id": {{ webpageId | jsonValue | safe }},
        "url": {{ canonicalUrl | jsonValue | safe }},
        "name": {{ currentTitle | jsonValue | safe }},{% if isDocsPage or isBlogPost %}
        "headline": {{ currentTitle | jsonValue | safe }},{% endif %}
        "description": {{ currentDescription | jsonValue | safe }},
        "isPartOf": { "@id": {{ websiteId | jsonValue | safe }} },
        "inLanguage": {{ effLang | jsonValue | safe }}{% if not isHome %},
        "breadcrumb": { "@id": {{ breadcrumbId | jsonValue | safe }} }{% endif %}{% if isBlogPost %},
        "datePublished": {{ date | dateToRfc3339 | jsonValue | safe }},
        "dateModified": {{ (updated | default(date)) | dateToRfc3339 | jsonValue | safe }},
        "image": {{ (site.url + pageOgImage) | jsonValue | safe }}{% endif %}{% if author and isBlogPost %},
        "author": {
          "@type": "Person",
          "name": {{ author | jsonValue | safe }}
        }{% endif %}{% if isDocsPage or isBlogPost %},
        "publisher": { "@id": {{ organizationId | jsonValue | safe }} }{% endif %}
      },
      {
        "@type": "Organization",
        "@id": {{ organizationId | jsonValue | safe }},
        "name": "Nimblesite",
        "url": "https://nimblesite.co",
        "sameAs": ["https://github.com/Nimblesite"]
      }{% if not isHome %},
      {
        "@type": "BreadcrumbList",
        "@id": {{ breadcrumbId | jsonValue | safe }},
        "itemListElement": [
          {
            "@type": "ListItem",
            "position": 1,
            "name": {{ homeName | jsonValue | safe }},
            "item": {{ homeUrl | jsonValue | safe }}
          }{% if sectionName %},
          {
            "@type": "ListItem",
            "position": 2,
            "name": {{ sectionName | jsonValue | safe }},
            "item": {{ sectionUrl | jsonValue | safe }}
          }{% endif %}{% if not sectionName or isBlogPost or (isDocsPage and basePath != '/docs/') %},
          {
            "@type": "ListItem",
            "position": {% if sectionName %}3{% else %}2{% endif %},
            "name": {{ currentTitle | jsonValue | safe }},
            "item": {{ canonicalUrl | jsonValue | safe }}
          }{% endif %}
        ]
      }{% endif %}
    ]
  }
  </script>

  <link rel="stylesheet" href="/techdoc/css/reset.css">
  <link rel="stylesheet" href="/techdoc/css/layout.css">
  <link rel="stylesheet" href="/techdoc/css/utilities.css">

  {% if site.stylesheet %}<link rel="stylesheet" href="{{ site.stylesheet }}">{% endif %}
  {% block head %}{% endblock %}
</head>
<body class="{% if basePath.startsWith('/docs/') or docsShell %}is-docs{% endif %}{% if bodyClass %} {{ bodyClass }}{% endif %}">
  <a href="#main-content" class="skip-link">{{ 'a11y.skipToContent' | t(effLang) | default('Skip to main content') }}</a>

  <header class="site-header">
    <nav class="nav">
      <div class="logo-wrap">
        <a href="{% if effLang != defaultLanguage %}/{{ effLang }}/{% else %}/{% endif %}" class="logo">
          <img src="/assets/img/logo.svg" alt="" class="logo-mark" width="28" height="28" aria-hidden="true">
          <span class="logo-word">{{ site.name | default(site.title) }}</span>
        </a>
        <span class="logo-badge" aria-label="{{ 'a11y.liveServer' | t(effLang) | default('Live analysis server') }}">
          <span class="logo-badge__dot" aria-hidden="true"></span>live
        </span>
      </div>

      <ul class="nav-links">
        {% set navData = navigation %}
        {% set currentLang = effLang %}
        {% for item in navData.main %}
        <li>
          {% set navUrl = item.url %}
          {% if not item.external and not item.noLangPrefix and currentLang != defaultLanguage %}
            {% set navUrl = item.url | altLangUrl('en', currentLang) %}
          {% endif %}
          {% set isCurrentSection = (item.url == '/' and (page.url == '/' or page.url == '/index.html' or page.url == ('/' + currentLang + '/') or page.url == ('/' + currentLang + '/index.html'))) or (item.url != '/' and item.url | length > 1 and (page.url.startsWith(item.url) or page.url.startsWith(navUrl))) %}
          <a href="{{ navUrl }}" {% if item.external %}target="_blank" rel="noopener noreferrer"{% endif %}
             class="nav-link{% if isCurrentSection %} active{% endif %}"{% if isCurrentSection %} aria-current="{% if page.url == navUrl %}page{% else %}location{% endif %}"{% endif %}>
            {% if item.i18nKey and currentLang != defaultLanguage %}{{ item.i18nKey | t(currentLang) | default(item.text) }}{% else %}{{ item.text }}{% endif %}
          </a>
        </li>
        {% endfor %}
      </ul>

      <div class="nav-actions">
        <div class="language-switcher">
          {%- if not noTranslation %}
          {% if effLang == 'zh' %}
          <a href="{{ basePath }}" lang="en" class="lang-link" aria-label="Switch to English">English</a>
          {% else %}
          <a href="/zh{{ basePath }}" lang="zh" class="lang-link" aria-label="切换到中文">中文</a>
          {% endif %}
          {%- endif %}
        </div>

        {% if techdocOptions.features.darkMode %}
        <button id="theme-toggle" class="theme-toggle" aria-label="{{ 'a11y.toggleTheme' | t(effLang) | default('Toggle dark mode') }}">
          <span class="theme-icon-light">☀</span>
          <span class="theme-icon-dark">☾</span>
        </button>
        {% endif %}

        <button id="mobile-menu-toggle" class="mobile-menu-toggle" aria-label="{{ 'a11y.toggleMenu' | t(effLang) | default('Toggle menu') }}">
          <span></span>
          <span></span>
          <span></span>
        </button>
      </div>
    </nav>
  </header>

  <div class="drawer-scrim" data-drawer-close></div>

  <main id="main-content">
    {% block content %}{{ content | safe }}{% endblock %}
  </main>

  {% if not hideFooter %}<footer class="site-footer">
    <div class="footer-content">
      {% if navigation.footer %}
      <div class="footer-grid">
        {% set currentLang = effLang %}
        {% for section in navigation.footer %}
        <div class="footer-section">
          <h2>{% if section.i18nKey and currentLang != defaultLanguage %}{{ section.i18nKey | t(currentLang) | default(section.title) }}{% else %}{{ section.title }}{% endif %}</h2>
          <ul>
            {% for item in section.items %}
            {% set footerUrl = item.url %}
            {% if item.url.startsWith('/') and currentLang != defaultLanguage and not item.noLangPrefix %}
              {% set footerUrl = item.url | altLangUrl('en', currentLang) %}
            {% endif %}
            <li><a href="{{ footerUrl }}">{% if item.i18nKey and currentLang != defaultLanguage %}{{ item.i18nKey | t(currentLang) | default(item.text) }}{% else %}{{ item.text }}{% endif %}</a></li>
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
  </footer>{% endif %}

  <script src="/techdoc/js/main.js" type="module"></script>
  <script src="/assets/js/drawer.js" type="module"></script>

  {% block scripts %}{% endblock %}
</body>
</html>
`;
