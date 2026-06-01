// Single-page section switching.
//
// All three sections (Writing, Commonplace, Bibliography) are rendered into the home page as
// .panel elements; only one is shown at a time. The URL hash names the active section
// (/ = writing, /#commonplace, /#bibliography), so back/forward and deep links work. The
// switch animates transform + opacity only — both are compositor-only, so it never triggers
// layout and can't stutter. The container height snaps once at the start of each switch (by
// design — animating height would reintroduce per-frame layout). The slide direction follows
// the tab order, so moving to a tab on the right mirrors moving back to one on the left.
//
// Progressive enhancement: if this script fails to load, nav clicks fall back to full-page
// reloads and the inline head script still shows the hash-named section on arrival. With JS
// fully disabled, base.html's <noscript> rule shows all sections stacked.
(function () {
  var PANELS = ["writing", "commonplace", "bibliography"];
  var els = {};
  PANELS.forEach(function (name) { els[name] = document.getElementById(name); });

  // Not the single-page shell (e.g. a post): leave every link to normal navigation.
  if (!els.writing || !els.commonplace || !els.bibliography) return;

  var root = document.documentElement;
  var view = document.getElementById("view");
  var reduceMotion = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function panelFor(url) {
    if (url.pathname !== "/") return null;            // only the home page hosts the panels
    var name = url.hash.replace(/^#/, "");
    return els[name] ? name : "writing";
  }

  function setActiveNav(name) {
    document.querySelectorAll(".site-nav a").forEach(function (a) {
      if (a.getAttribute("data-panel") === name) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    });
  }

  var current = panelFor(new URL(location.href)) || "writing";

  // Hand off from the declarative data-panel state to class-driven state, without animating
  // the first render.
  PANELS.forEach(function (name) {
    els[name].className = name === current ? "panel is-active" : "panel";
  });
  root.removeAttribute("data-panel");
  setActiveNav(current);

  function onceAnimEnd(el, fn) {
    el.addEventListener("animationend", function handler() {
      el.removeEventListener("animationend", handler);
      fn();
    });
  }

  function setPanel(name, animate) {
    if (!els[name] || name === current) return;
    var incoming = els[name];
    var outgoing = els[current];

    // Slide direction follows tab order: +1 toward a tab on the right, -1 toward the left.
    // Computed against the outgoing panel before `current` moves on.
    var dir = PANELS.indexOf(name) > PANELS.indexOf(current) ? 1 : -1;

    // Finalize any panel still mid-leave from a rapid previous switch.
    PANELS.forEach(function (n) {
      if (n !== current && n !== name) els[n].className = "panel";
    });

    current = name;
    setActiveNav(name);
    window.scrollTo(0, 0);

    if (!animate || reduceMotion) {
      outgoing.className = "panel";
      incoming.className = "panel is-active";
    } else {
      // Set direction before the classes that start the animation so the keyframes read it.
      // Only the incoming/outgoing pair animates (the finalize loop reset any third panel),
      // and both inherit --slide-dir from #view, so they always slide consistently.
      (view || root).style.setProperty("--slide-dir", String(dir));
      incoming.className = "panel is-active anim-in";
      outgoing.className = "panel is-leaving";
      onceAnimEnd(incoming, function () { incoming.classList.remove("anim-in"); });
      onceAnimEnd(outgoing, function () {
        if (els[current] !== outgoing) outgoing.className = "panel";
      });
    }

    // Focus only after the panel is shown — focusing a display:none element is a no-op.
    incoming.focus({ preventScroll: true });   // move focus to the new section for a11y
  }

  // Intercept in-page links that target the home page; everything else navigates normally.
  document.addEventListener("click", function (e) {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    var a = e.target.closest("a");
    if (!a || !a.getAttribute("href")) return;
    var url = new URL(a.href, location.href);
    if (url.origin !== location.origin) return;
    var name = panelFor(url);
    if (name === null) return;                        // link leaves the home page -> full nav
    e.preventDefault();
    var href = name === "writing" ? "/" : "/#" + name;
    if (name !== current) {
      history.pushState({ panel: name }, "", href);
      setPanel(name, true);
    } else if (location.hash || href === "/") {
      history.replaceState({ panel: name }, "", href);   // normalize URL without a switch
    }
  });

  window.addEventListener("popstate", function () {
    var name = panelFor(new URL(location.href)) || "writing";
    setPanel(name, true);
  });
})();
