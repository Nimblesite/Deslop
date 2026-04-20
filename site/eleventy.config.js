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
      <p class="docs-sidebar__version">v0.1.0-preview</p>
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
      <a href="https://github.com/MelbourneDeveloper/CodeDedup" class="docs-sidebar__cta">
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

const OVERRIDES = {
  "blog/index.njk": BLOG_INDEX_OVERRIDE,
  "_includes/layouts/docs.njk": DOCS_LAYOUT_OVERRIDE,
  "_includes/layouts/blog.njk": BLOG_POST_LAYOUT_OVERRIDE,
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
      name: "CodeDedup",
      url: "https://codededup.dev",
      description:
        "Duplicate-code detection for the AI era. Tree-sitter parsing, AST fingerprinting, and semantic fusion — ranked worst-offender first so agents and humans fix what matters.",
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

  return {
    dir: { input: "src", output: "_site" },
    markdownTemplateEngine: "njk",
  };
}
