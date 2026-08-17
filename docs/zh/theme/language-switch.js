(function () {
  "use strict";
  function targetUrl() {
    var p = window.location.pathname;
    if (p.indexOf("/zh/") !== -1) {
      return p.replace("/zh/", "/");
    }
    var idx = p.lastIndexOf("/");
    return p.slice(0, idx) + "/zh" + p.slice(idx);
  }
  var a = document.createElement("a");
  a.href = targetUrl();
  a.className = "language-switch";
  a.textContent =
    window.location.pathname.indexOf("/zh/") !== -1 ? "English" : "中文";
  document.body.appendChild(a);
})();
