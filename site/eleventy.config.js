import techdoc from "eleventy-plugin-techdoc";

const BLOG_INDEX_OVERRIDE = `---
layout: layouts/base.njk
title: Blog
permalink: /blog/
---
<div class="blog-container">
  <header class="blog-header">
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

/**
 * Replaces the plugin-registered virtual blog index with one that sorts by
 * date DESC. The plugin's template uses `| reverse` on the collection proxy,
 * which does not produce newest-first order.
 */
function overrideBlogIndexPlugin(eleventyConfig) {
  const vt = eleventyConfig.virtualTemplates;
  if (vt && vt["blog/index.njk"]) {
    vt["blog/index.njk"].content = BLOG_INDEX_OVERRIDE;
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

  // Runs AFTER the techdoc plugin queue has executed, giving us access to
  // `virtualTemplates` with the plugin's registrations populated.
  eleventyConfig.addPlugin(overrideBlogIndexPlugin);

  eleventyConfig.addPassthroughCopy("src/assets");

  return {
    dir: { input: "src", output: "_site" },
    markdownTemplateEngine: "njk",
  };
}
