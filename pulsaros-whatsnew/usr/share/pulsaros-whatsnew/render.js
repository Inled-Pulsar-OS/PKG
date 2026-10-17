/*
 * Pulsar OS - What's New
 * Small, dependency-free Markdown renderer for the release notes viewer.
 * It intentionally supports the subset used by releases.d/ entries:
 * headings, paragraphs, emphasis, inline code, fenced code, links, images,
 * lists, blockquotes, horizontal rules and tables.
 */
(function () {
  "use strict";

  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function inline(text) {
    var t = escapeHtml(text);
    // images
    t = t.replace(
      /!\[([^\]]*)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/g,
      function (_m, alt, url, title) {
        return (
          '<img src="' + url + '" alt="' + alt + '"' +
          (title ? ' title="' + title + '"' : "") + ">"
        );
      }
    );
    // links
    t = t.replace(
      /\[([^\]]+)\]\(([^)\s]+)(?:\s+"([^"]*)")?\)/g,
      function (_m, label, url, title) {
        return (
          '<a href="' + url + '"' + (title ? ' title="' + title + '"' : "") +
          ">" + label + "</a>"
        );
      }
    );
    // autolinks
    t = t.replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, function (_m, pre, url) {
      return pre + '<a href="' + url + '">' + url + "</a>";
    });
    // inline code (before emphasis so * inside code is preserved)
    t = t.replace(/`([^`]+)`/g, function (_m, code) {
      return "<code>" + code + "</code>";
    });
    // bold / italic / strike
    t = t.replace(/\*\*\*([^*]+)\*\*\*/g, "<strong><em>$1</em></strong>");
    t = t.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
    t = t.replace(/(^|[^*])\*([^*\n]+)\*(?!\*)/g, "$1<em>$2</em>");
    t = t.replace(/__([^_]+)__/g, "<strong>$1</strong>");
    t = t.replace(/(^|[^_])_([^_\n]+)_(?!_)/g, "$1<em>$2</em>");
    t = t.replace(/~~([^~]+)~~/g, "<del>$1</del>");
    return t;
  }

  function splitRow(line) {
    return line
      .trim()
      .replace(/^\|/, "")
      .replace(/\|$/, "")
      .split("|")
      .map(function (cell) {
        return cell.trim();
      });
  }

  function isHr(line) {
    return /^\s*([-*_])\s*\1\s*\1[\s\1]*$/.test(line);
  }

  function isTableSeparator(line) {
    return /^\s*\|?[\s:|-]+\|[\s:|-]*$/.test(line) && line.indexOf("-") !== -1;
  }

  function renderMarkdown(src) {
    src = String(src || "").replace(/\r\n?/g, "\n");
    var lines = src.split("\n");
    var out = [];
    var para = [];
    var i = 0;

    function flushPara() {
      if (para.length) {
        out.push("<p>" + inline(para.join(" ")) + "</p>");
        para = [];
      }
    }

    while (i < lines.length) {
      var line = lines[i];

      // Fenced code block
      var fence = line.match(/^\s*```(\w*)\s*$/);
      if (fence) {
        flushPara();
        var lang = fence[1];
        i++;
        var buf = [];
        while (i < lines.length && !/^\s*```\s*$/.test(lines[i])) {
          buf.push(lines[i]);
          i++;
        }
        i++;
        out.push(
          "<pre><code" + (lang ? ' class="language-' + lang + '"' : "") + ">" +
          escapeHtml(buf.join("\n")) + "</code></pre>"
        );
        continue;
      }

      // Heading
      var heading = line.match(/^(#{1,6})\s+(.*)$/);
      if (heading) {
        flushPara();
        var level = heading[1].length;
        out.push("<h" + level + ">" + inline(heading[2]) + "</h" + level + ">");
        i++;
        continue;
      }

      // Horizontal rule
      if (isHr(line)) {
        flushPara();
        out.push("<hr>");
        i++;
        continue;
      }

      // Blockquote
      if (/^\s*>\s?/.test(line)) {
        flushPara();
        var quote = [];
        while (i < lines.length && /^\s*>\s?/.test(lines[i])) {
          quote.push(lines[i].replace(/^\s*>\s?/, ""));
          i++;
        }
        out.push("<blockquote>" + renderMarkdown(quote.join("\n")) + "</blockquote>");
        continue;
      }

      // Table
      if (
        line.indexOf("|") !== -1 &&
        i + 1 < lines.length &&
        isTableSeparator(lines[i + 1])
      ) {
        flushPara();
        var header = splitRow(line);
        i += 2;
        var rows = [];
        while (i < lines.length && lines[i].indexOf("|") !== -1 && lines[i].trim() !== "") {
          rows.push(splitRow(lines[i]));
          i++;
        }
        var table = "<table><thead><tr>";
        header.forEach(function (cell) {
          table += "<th>" + inline(cell) + "</th>";
        });
        table += "</tr></thead><tbody>";
        rows.forEach(function (row) {
          table += "<tr>";
          header.forEach(function (_cell, idx) {
            table += "<td>" + inline(row[idx] || "") + "</td>";
          });
          table += "</tr>";
        });
        table += "</tbody></table>";
        out.push(table);
        continue;
      }

      // Lists
      var ul = line.match(/^(\s*)([-*+])\s+(.*)$/);
      var ol = line.match(/^(\s*)(\d+)[.)]\s+(.*)$/);
      if (ul || ol) {
        flushPara();
        var ordered = !!ol;
        var items = [];
        while (i < lines.length) {
          var mU = lines[i].match(/^(\s*)([-*+])\s+(.*)$/);
          var mO = lines[i].match(/^(\s*)(\d+)[.)]\s+(.*)$/);
          if (ordered ? mO : mU || mO) {
            items.push((mU || mO)[3]);
            i++;
          } else if (/^\s+\S/.test(lines[i]) && items.length) {
            items[items.length - 1] += " " + lines[i].trim();
            i++;
          } else {
            break;
          }
        }
        var tag = ordered ? "ol" : "ul";
        out.push(
          "<" + tag + ">" +
          items
            .map(function (item) {
              return "<li>" + inline(item) + "</li>";
            })
            .join("") +
          "</" + tag + ">"
        );
        continue;
      }

      if (line.trim() === "") {
        flushPara();
        i++;
        continue;
      }

      para.push(line.trim());
      i++;
    }

    flushPara();
    return out.join("\n");
  }

  function renderNotes(notes) {
    return notes
      .map(function (note) {
        var head = "";
        if (note.title) {
          head += "<h1>" + escapeHtml(note.title) + "</h1>";
        }
        var meta = [];
        if (note.date) meta.push(escapeHtml(note.date));
        if (note.channel) meta.push(escapeHtml(note.channel));
        if (note.important) meta.push("destacado");
        var metaHtml = meta.length
          ? '<p class="note-meta">' + meta.join(" · ") + "</p>"
          : "";
        var pkgs = "";
        if (note.packages && note.packages.length) {
          pkgs =
            '<p class="note-packages">' +
            note.packages
              .map(function (p) {
                return "<code>" + escapeHtml(p) + "</code>";
              })
              .join(" ") +
            "</p>";
        }
        return (
          '<article class="note" id="note-' + escapeHtml(note.id) + '">' +
          head + metaHtml + pkgs +
          '<div class="note-body">' + renderMarkdown(note.markdown) + "</div>" +
          "</article>"
        );
      })
      .join('<hr class="note-sep">');
  }

  function boot() {
    var payloadEl = document.getElementById("payload");
    var content = document.getElementById("content");
    if (!payloadEl || !content) return;
    var payload;
    try {
      payload = JSON.parse(payloadEl.textContent);
    } catch (err) {
      content.innerHTML = "<p>Could not read the release notes.</p>";
      return;
    }
    content.innerHTML = renderNotes(payload.notes || []);
  }

  window.renderMarkdown = renderMarkdown;
  window.renderNotes = renderNotes;
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
