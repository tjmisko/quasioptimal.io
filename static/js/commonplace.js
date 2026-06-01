// Progressive enhancement only: the page is fully rendered server-side. This swaps the
// server-rendered featured quote (the first entry) for a random one on each visit, so the
// "drawn at random each visit" promise holds for JS users and degrades cleanly without it.
(function () {
  var quotes = document.querySelectorAll("#commonplace-list .quote");
  var slot = document.getElementById("commonplace-featured");
  if (!quotes.length || !slot) return;
  var pick = quotes[Math.floor(Math.random() * quotes.length)].cloneNode(true);
  pick.classList.add("quote-large");
  slot.replaceChildren(pick);
})();
