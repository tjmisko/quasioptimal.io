// Progressive-enhancement SPA navigation.
//
// Zola builds /, /bibliography/, and /commonplace/ as ordinary pages. On these "shell"
// pages we swap the #view container instead of doing a full reload, and use the History
// API so the address bar shows the real path (not a #hash). Posts and external links
// navigate normally, and with JS disabled everything is just plain links — no regressions.
(function () {
  if (!window.history || !window.fetch || !window.DOMParser) return;

  var SHELL = ["/", "/bibliography/", "/commonplace/"];
  var isShell = function (path) { return SHELL.indexOf(path) !== -1; };
  var view = function () { return document.getElementById("view"); };

  // Prefetch cache: warm it on hover/focus so the click itself has nothing to wait for.
  var cache = new Map();
  function fetchPage(path) {
    if (!cache.has(path)) {
      cache.set(path, fetch(path, { headers: { "X-Requested-With": "fetch" } })
        .then(function (res) {
          if (!res.ok) throw new Error(res.status);
          return res.text();
        })
        .catch(function (err) { cache.delete(path); throw err; }));
    }
    return cache.get(path);
  }

  function setActiveNav(pathname) {
    document.querySelectorAll(".site-nav a").forEach(function (a) {
      if (new URL(a.href).pathname === pathname) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    });
  }

  function runScripts(container) {
    // Script nodes inserted via DOM swap don't execute on their own; re-create them so
    // per-view behaviour (e.g. the commonplace random-featured picker) runs after a swap.
    container.querySelectorAll("script").forEach(function (old) {
      var s = document.createElement("script");
      for (var i = 0; i < old.attributes.length; i++) {
        s.setAttribute(old.attributes[i].name, old.attributes[i].value);
      }
      if (!old.src) s.textContent = old.textContent;
      old.replaceWith(s);
    });
  }

  async function navigate(path, push) {
    var current = view();
    if (!current) { location.assign(path); return; }

    var html;
    try {
      html = await fetchPage(path);
    } catch (e) { location.assign(path); return; }   // network/HTTP error: fall back to full load

    var doc = new DOMParser().parseFromString(html, "text/html");
    var next = doc.getElementById("view");
    if (!next) { location.assign(path); return; }

    if (push) history.pushState({ spa: true }, "", path);

    var swap = function () {
      current.replaceWith(next);
      document.title = doc.title;
      setActiveNav(path);
      runScripts(next);
      window.scrollTo(0, 0);
      next.focus({ preventScroll: true });   // move focus to the new content for a11y
    };

    if (document.startViewTransition) document.startViewTransition(swap);
    else swap();
  }

  document.addEventListener("click", function (e) {
    if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    if (!document.querySelector(".site-nav")) return;   // not on a shell page (e.g. a post) -> leave links alone
    var a = e.target.closest("a");
    if (!a || !a.getAttribute("href")) return;
    var url = new URL(a.href, location.href);
    if (url.origin !== location.origin) return;         // external link
    if (!isShell(url.pathname)) return;                 // post / other -> normal full navigation
    e.preventDefault();
    if (url.pathname !== location.pathname) navigate(url.pathname, true);
  });

  function maybePrefetch(target) {
    var a = target && target.closest && target.closest("a");
    if (!a || !a.getAttribute("href")) return;
    var url = new URL(a.href, location.href);
    if (url.origin === location.origin && isShell(url.pathname)) fetchPage(url.pathname).catch(function () {});
  }
  document.addEventListener("mouseover", function (e) { maybePrefetch(e.target); });
  document.addEventListener("focusin", function (e) { maybePrefetch(e.target); });

  window.addEventListener("popstate", function () {
    if (isShell(location.pathname)) navigate(location.pathname, false);
  });
})();
