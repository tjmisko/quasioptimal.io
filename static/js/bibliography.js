// Bibliography blurbs: click-to-expand (progressive enhancement).
//
// Each source entry's blurb (.biblio-note) is rendered visible server-side. This script marks
// every entry that has one as collapsible; CSS then keeps it collapsed (the `js` class is set in
// the head before first paint, so there is no flash) and animates it open when the entry's
// citation line is clicked. A click on the title link navigates as normal — only clicks
// elsewhere on the line toggle. If this script never runs, the blurb simply stays in its
// collapsed state and the citation itself, the essential content, remains fully visible.
(function () {
  var entries = document.querySelectorAll("#bibliography .biblio-entry");
  if (!entries.length) return; // not the page that hosts the bibliography

  var uid = 0;

  entries.forEach(function (entry) {
    var note = entry.querySelector(".biblio-note");
    var line = entry.querySelector(".biblio-line");
    if (!note || !line) return; // entries without a blurb aren't interactive

    entry.classList.add("is-collapsible");

    // Wire the citation line as a disclosure control for the blurb.
    if (!note.id) note.id = "biblio-note-" + ++uid;
    line.setAttribute("role", "button");
    line.setAttribute("tabindex", "0");
    line.setAttribute("aria-expanded", "false");
    line.setAttribute("aria-controls", note.id);

    function toggle() {
      var open = entry.classList.toggle("is-open");
      line.setAttribute("aria-expanded", open ? "true" : "false");
    }

    // A click on (or inside) the title link should follow the link, not toggle.
    line.addEventListener("click", function (e) {
      if (e.target.closest("a")) return;
      toggle();
    });

    // Keyboard: Enter / Space toggle, unless focus is on the inner link.
    line.addEventListener("keydown", function (e) {
      if (e.target.closest("a")) return;
      if (e.key === "Enter" || e.key === " " || e.key === "Spacebar") {
        e.preventDefault();
        toggle();
      }
    });
  });
})();
