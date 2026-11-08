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

  function renderContributors(contributors) {
    if (!contributors || !contributors.length) return "";
    var cards = contributors.map(function(c) {
      var discordBtn = "";
      if (c.discord) {
        discordBtn =
          '<a href="' + escapeHtml(c.discord) + '" target="_blank" class="social-btn discord-btn" title="Discord: @' + escapeHtml(c.discordUser || c.name) + '">' +
            '<svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.929 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.894.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>' +
          '</a>';
      }
      var githubBtn = "";
      if (c.github) {
        githubBtn =
          '<a href="' + escapeHtml(c.github) + '" target="_blank" class="social-btn github-btn" title="GitHub: ' + escapeHtml(c.name) + '">' +
            '<svg viewBox="0 0 24 24" width="15" height="15" fill="currentColor"><path fill-rule="evenodd" clip-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"/></svg>' +
          '</a>';
      }
      var roleHtml = "";
      if (c.role) {
        roleHtml = '<div class="contributor-role" title="' + escapeHtml(c.role) + '">' + escapeHtml(c.role) + '</div>';
      }
      return (
        '<div class="contributor-card">' +
          '<a href="' + escapeHtml(c.github || c.discord || "#") + '" target="_blank" class="contributor-avatar-link">' +
            '<img class="contributor-avatar" src="' + escapeHtml(c.avatar) + '" alt="' + escapeHtml(c.name) + '" loading="lazy">' +
          '</a>' +
          '<div class="contributor-name" title="' + escapeHtml(c.name) + '">' + escapeHtml(c.name) + '</div>' +
          roleHtml +
          '<div class="contributor-socials">' + discordBtn + githubBtn + '</div>' +
        '</div>'
      );
    }).join("");

    return (
      '<section class="contributors-section">' +
        '<hr class="note-sep">' +
        '<div class="contributors-header">' +
          '<h2>The people behind Pulsar OS</h2>' +
          '<p class="contributors-desc">When you contribute to Pulsar OS, no matter how, you get recognized. Join our community!</p>' +
        '</div>' +
        '<div class="contributors-grid">' + cards + '</div>' +
      '</section>'
    );
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
    var contribEl = document.getElementById("contributors-payload");
    var contributors = [];
    if (contribEl && contribEl.textContent.trim()) {
      try {
        contributors = JSON.parse(contribEl.textContent);
      } catch (err) {}
    }
    content.innerHTML = renderNotes(payload.notes || []) + renderContributors(contributors);
  }

  window.renderMarkdown = renderMarkdown;
  window.renderNotes = renderNotes;
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
