// Renders data/bibliography.json into topic-grouped sections.
// No dependencies. Requires being served over http (fetch); see <noscript> fallback.

const MOUNT = document.getElementById("bibliography");

function surname(author) {
  const parts = String(author || "").trim().split(/\s+/);
  return parts.length ? parts[parts.length - 1].toLowerCase() : "";
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text != null) node.textContent = text;
  return node;
}

function renderTitle(entry) {
  // Books are italicised; articles are quoted — matching the site's prose conventions.
  const isBook = entry.type === "book";
  const inner = isBook ? el("cite", "biblio-book-title", entry.title)
                       : document.createTextNode("“" + entry.title + "”");
  if (entry.url) {
    const link = el("a", "biblio-title-link");
    link.href = entry.url;
    link.rel = "noopener";
    link.appendChild(inner);
    return link;
  }
  const span = el("span", "biblio-title");
  span.appendChild(inner);
  return span;
}

function renderEntry(entry) {
  const wrap = el("div", "biblio-entry");

  const line = el("p", "biblio-line");
  line.appendChild(renderTitle(entry));

  const meta = [];
  if (entry.author) meta.push(entry.author);
  if (entry.year != null) meta.push(String(entry.year));
  if (meta.length) {
    line.appendChild(document.createTextNode(" "));
    line.appendChild(el("span", "biblio-meta", "— " + meta.join(", ")));
  }
  if (entry.type) {
    line.appendChild(document.createTextNode(" "));
    line.appendChild(el("span", "biblio-type", entry.type));
  }
  wrap.appendChild(line);

  if (entry.note) wrap.appendChild(el("p", "biblio-note", entry.note));
  return wrap;
}

function render(entries) {
  if (!Array.isArray(entries) || entries.length === 0) {
    MOUNT.appendChild(el("p", "empty-state", "No entries yet."));
    return;
  }

  const byTopic = new Map();
  for (const entry of entries) {
    const topic = entry.topic || "Uncategorized";
    if (!byTopic.has(topic)) byTopic.set(topic, []);
    byTopic.get(topic).push(entry);
  }

  const topics = [...byTopic.keys()].sort((a, b) => a.localeCompare(b));
  for (const topic of topics) {
    const section = el("section", "biblio-topic-section");
    section.appendChild(el("h2", "biblio-topic", topic));

    const items = byTopic.get(topic).sort((a, b) => {
      const bySurname = surname(a.author).localeCompare(surname(b.author));
      return bySurname !== 0 ? bySurname : (a.year || 0) - (b.year || 0);
    });
    for (const entry of items) section.appendChild(renderEntry(entry));
    MOUNT.appendChild(section);
  }
}

fetch("data/bibliography.json")
  .then((res) => {
    if (!res.ok) throw new Error("HTTP " + res.status);
    return res.json();
  })
  .then(render)
  .catch((err) => {
    MOUNT.appendChild(
      el("p", "empty-state", "Could not load the bibliography. Serve the site over http and try again.")
    );
    console.error("bibliography:", err);
  });
