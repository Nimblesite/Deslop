// Strip the upstream `/api/` link from llms.txt — Deslop's site doesn't
// ship an API reference route, so the placeholder would be a 404 from the
// AI-discoverability artefact itself.
export const LLMS_TXT_OVERRIDE = `---json
{
  "permalink": "llms.txt",
  "eleventyExcludeFromCollections": true
}
---
# {{ site.title | default(site.name) | safe }}

> {{ site.description | safe }}

## Install

- VS Code extension (preferred — bundles LSP + MCP + CLI): https://github.com/Nimblesite/Deslop/releases/latest
- Release index: {{ site.url }}/releases/
- Homebrew (CLI only): brew install nimblesite/tap/deslop
- Scoop (CLI only): scoop install deslop

## Documentation
{% for page in collections.docs %}
- [{{ page.data.title | safe }}]({{ site.url }}{{ page.url }}){% if page.data.description %} — {{ page.data.description | safe }}{% endif %}
{% endfor %}

## Blog Posts
{% for post in collections.posts | reverse | limit(10) %}
- [{{ post.data.title | safe }}]({{ site.url }}{{ post.url }}){% if post.data.excerpt or post.data.description %} — {{ post.data.excerpt | default(post.data.description) | safe }}{% endif %}
{% endfor %}

## Navigation
- Home: {{ site.url }}/
- Documentation: {{ site.url }}/docs/
- Blog: {{ site.url }}/blog/
- Releases: {{ site.url }}/releases/
- Source: https://github.com/Nimblesite/Deslop
`;

// A concise Atom feed is more useful than embedding every article body. It
// also avoids passing whole HTML documents through a URL filter, which turns
// the body into an encoded URL instead of valid feed content.
export const FEED_OVERRIDE = `---json
{
  "permalink": "feed.xml",
  "eleventyExcludeFromCollections": true
}
---
<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en">
  <title>{{ site.title }}</title>
  <subtitle>{{ site.description }}</subtitle>
  <link href="{{ site.url }}/feed.xml" rel="self"/>
  <link href="{{ site.url }}/"/>
  <updated>{% if collections.posts | length > 0 %}{{ collections.posts | getNewestCollectionItemDate | dateToRfc3339 }}{% else %}1970-01-01T00:00:00.000Z{% endif %}</updated>
  <id>{{ site.url }}/</id>
  {% if site.author %}<author><name>{{ site.author }}</name></author>{% endif %}
  {%- for post in collections.posts | reverse %}
  <entry>
    <title>{{ post.data.title }}</title>
    <link href="{{ site.url }}{{ post.url }}"/>
    <id>{{ site.url }}{{ post.url }}</id>
    <published>{{ post.date | dateToRfc3339 }}</published>
    <updated>{{ (post.data.updated | default(post.date)) | dateToRfc3339 }}</updated>
    <summary type="text">{{ post.data.excerpt | default(post.data.description) }}</summary>
  </entry>
  {%- endfor %}
</feed>
`;

// Only real publication dates belong in sitemap lastmod. Eleventy's implicit
// file date changes with checkout/build time and would make evergreen pages
// look freshly published on every build.
export const SITEMAP_OVERRIDE = `---json
{
  "permalink": "sitemap.xml",
  "eleventyExcludeFromCollections": true
}
---
<?xml version="1.0" encoding="utf-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{%- for item in collections.all %}
  {% if item.url and item.url != '/404.html' %}
  <url>
    <loc>{{ site.url }}{{ item.url }}</loc>{% if item.data.tags and ('posts' in item.data.tags) %}
    <lastmod>{{ (item.data.updated | default(item.date)) | dateToRfc3339 }}</lastmod>{% endif %}
  </url>
  {% endif %}
{%- endfor %}
</urlset>
`;

// Seven posts do not justify 26 thin tag/category archives, including several
// one-post pages and duplicate Go/Golang archives. Keep the useful post
// metadata but stop publishing the low-value taxonomy pages.
export const DISABLED_ARCHIVE_TEMPLATE = `---json
{
  "permalink": false,
  "eleventyExcludeFromCollections": true
}
---
`;

// robots.txt override: the plugin default `Disallow: /assets/` blocks every
// crawler in the `*` group (which includes social-card fetchers like
// facebookexternalhit, Twitterbot, LinkedInBot, and Slackbot — none are in the
// named allow-list below) from reading the Open Graph images under
// /assets/img/, so link previews render without a card. It also hides CSS/JS,
// which Google's guidance says never to block since Googlebot needs them to
// render. Static assets carry no crawl risk, so only the search endpoint is
// disallowed. The named search/AI-crawler groups are kept verbatim.
export const ROBOTS_TXT_OVERRIDE = `---json
{
  "permalink": "robots.txt",
  "eleventyExcludeFromCollections": true
}
---
# {{ site.title | default(site.name) }}
# {{ site.url }}

# Allow all crawlers
User-agent: *
Allow: /
Disallow: /search?

# AI Crawlers - Welcome!
User-agent: GPTBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: Google-Extended
Allow: /

User-agent: Googlebot
Allow: /

User-agent: Bingbot
Allow: /

User-agent: ClaudeBot
Allow: /

User-agent: Anthropic-AI
Allow: /

User-agent: PerplexityBot
Allow: /

User-agent: Cohere-ai
Allow: /

User-agent: Meta-ExternalAgent
Allow: /

User-agent: Meta-ExternalFetcher
Allow: /

User-agent: Bytespider
Allow: /

User-agent: CCBot
Allow: /

User-agent: Applebot
Allow: /

User-agent: Amazonbot
Allow: /

# Sitemaps
Sitemap: {{ site.url }}/sitemap.xml
`;
