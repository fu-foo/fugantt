// The little the server-drawn pages need a script for.
//
// These used to be `onclick` attributes. The content policy keeps the page to
// scripts it loads from here, which leaves no room for code written into the
// markup — so the markup says what it wants, and this does it.

// A button that asks first: data-confirm="…". Saying no stops the form.
document.addEventListener("click", (event) => {
  const target = event.target instanceof Element ? event.target : null;

  const asking = target?.closest("[data-confirm]");
  if (asking && !window.confirm(asking.getAttribute("data-confirm") ?? "")) {
    event.preventDefault();
    return;
  }

  // The error page's 「前の画面へ戻る」.
  if (target?.closest("[data-back]")) {
    event.preventDefault();
    history.back();
  }
});
