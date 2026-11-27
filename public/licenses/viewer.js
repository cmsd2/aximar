// The Licences window: the tabs along the top stay put, and each notice
// loads below them, so there is always a way back to the overview.  The
// files are in the app bundle; see docs/licences.md.
(function () {
  "use strict";

  var view = document.getElementById("view");
  var overview = document.getElementById("overview");
  var tabs = document.querySelectorAll(".bar nav a");
  /** Counts loads, so only the latest one shows */
  var latest = 0;

  /** The notice files each tab shows */
  var VIEWS = {
    npm: [{ file: "third-party-npm.html", html: true, built: true }],
    rust: [{ file: "third-party-rust.html", html: true }],
    fonts: [
      { file: "JetBrainsMono-OFL.txt", title: "JetBrains Mono" },
      { file: "ComputerModern-OFL.txt", title: "Computer Modern Unicode" },
    ],
  };

  function load(part) {
    return fetch(part.file).then(function (response) {
      if (!response.ok) throw new Error(part.file + ": " + response.status);
      return response.text().then(function (text) {
        // A development server answers a missing file with the app's own
        // page; a notices page carries a marker, and a text file isn't HTML
        var looksLikeHtml = /^\s*<(!doctype|html)/i.test(text);
        var ours = /<meta name="aximar-notices"/.test(text);
        if (part.html ? !ours : looksLikeHtml) throw new Missing(part);
        return text;
      });
    });
  }

  function Missing(part) {
    this.part = part;
    this.message = part.built
      ? "These notices are written when the app is built (npm run build), so a development build doesn't have them."
      : part.file + " is missing from this build.";
  }

  /** A part as nodes: a text file as preformatted text, an HTML file's body */
  function render(part, text) {
    var out = [];
    if (part.title) {
      var heading = document.createElement("h2");
      heading.textContent = part.title;
      out.push(heading);
    }
    if (part.html) {
      var doc = new DOMParser().parseFromString(text, "text/html");
      Array.prototype.forEach.call(doc.body.children, function (child) {
        if (child.tagName !== "SCRIPT") out.push(document.importNode(child, true));
      });
    } else {
      var pre = document.createElement("pre");
      pre.textContent = text;
      out.push(pre);
    }
    return out;
  }

  function show(name) {
    var mine = ++latest;
    if (!VIEWS[name]) name = "overview";
    Array.prototype.forEach.call(tabs, function (tab) {
      if (tab.getAttribute("href") === "#" + name) tab.setAttribute("aria-current", "page");
      else tab.removeAttribute("aria-current");
    });
    view.replaceChildren();
    window.scrollTo(0, 0);
    if (name === "overview") {
      view.appendChild(overview);
      return Promise.resolve();
    }
    var parts = VIEWS[name];
    return Promise.all(parts.map(load))
      .then(function (texts) {
        // Another load started while this one ran
        if (mine !== latest) return;
        texts.forEach(function (text, i) {
          render(parts[i], text).forEach(function (node) {
            view.appendChild(node);
          });
        });
      })
      .catch(function (error) {
        if (mine !== latest) return;
        var p = document.createElement("p");
        p.textContent = error instanceof Missing ? error.message : "Couldn't load this notice: " + error.message;
        view.appendChild(p);
      });
  }

  window.addEventListener("hashchange", function () {
    show(location.hash.slice(1));
  });
  window.__licencesShown = show(location.hash.slice(1));
})();
