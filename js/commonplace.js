// Renders data/commonplace.json: one random featured quote plus the full collection.
// No dependencies. Requires being served over http (fetch); see <noscript> fallback.

const FEATURED = document.getElementById("commonplace-featured");
const LIST = document.getElementById("commonplace-list");

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text != null) node.textContent = text;
  return node;
}

function attribution(entry) {
  // "— Author, Source" with Source linked when a url is present.
  const attr = el("p", "quote-attr");
  if (entry.author) attr.appendChild(document.createTextNode("— " + entry.author));
  if (entry.source) {
    attr.appendChild(document.createTextNode(entry.author ? ", " : "— "));
    if (entry.url) {
      const link = el("a", null);
      link.href = entry.url;
      link.rel = "noopener";
      link.appendChild(el("cite", null, entry.source));
      attr.appendChild(link);
    } else {
      attr.appendChild(el("cite", null, entry.source));
    }
  }
  return attr;
}

function renderQuote(entry, className) {
  const fig = el("figure", className);
  const block = el("blockquote");
  block.appendChild(el("p", null, entry.quote));
  fig.appendChild(block);
  if (entry.author || entry.source) fig.appendChild(attribution(entry));
  if (entry.note) fig.appendChild(el("p", "quote-note", entry.note));
  return fig;
}

function render(entries) {
  if (!Array.isArray(entries) || entries.length === 0) {
    LIST.appendChild(el("p", "empty-state", "No quotes yet."));
    return;
  }

  const featuredIndex = Math.floor(Math.random() * entries.length);
  FEATURED.appendChild(renderQuote(entries[featuredIndex], "quote quote-large"));

  LIST.appendChild(el("h2", "commonplace-all-heading", "All entries"));
  for (const entry of entries) LIST.appendChild(renderQuote(entry, "quote"));
}

fetch("data/commonplace.json")
  .then((res) => {
    if (!res.ok) throw new Error("HTTP " + res.status);
    return res.json();
  })
  .then(render)
  .catch((err) => {
    LIST.appendChild(
      el("p", "empty-state", "Could not load the commonplace. Serve the site over http and try again.")
    );
    console.error("commonplace:", err);
  });
