(function () {
  'use strict';

  var LINE_DELAY = 90;
  var TERMINAL_GAP = 300;

  function animateTerminal(terminal, onLineVisible) {
    var lines = terminal.querySelectorAll('.problem-line');
    lines.forEach(function (line, i) {
      setTimeout(function () {
        line.classList.add('visible');
        if (onLineVisible) onLineVisible(line);
      }, i * LINE_DELAY);
    });
    return lines.length * LINE_DELAY;
  }

  function animateStats() {
    var stats = document.querySelectorAll('.problem-stat');
    stats.forEach(function (stat, i) {
      setTimeout(function () {
        stat.classList.add('visible');
      }, i * 120);
    });
  }

  function init() {
    var container = document.getElementById('problem-terminals');
    if (!container) return;

    var chaosTerminal = container.querySelector('.problem-terminal--chaos');
    var unifiedTerminal = container.querySelector('.problem-terminal--unified');
    var arrow = document.getElementById('problem-arrow');
    var hasPlayed = false;

    // Reduced motion: show everything immediately
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      chaosTerminal.classList.add('visible');
      unifiedTerminal.classList.add('visible');
      if (arrow) arrow.classList.add('visible');
      container.querySelectorAll('.problem-line').forEach(function (line) {
        line.classList.add('visible');
      });
      document.querySelectorAll('.problem-stat').forEach(function (stat) {
        stat.classList.add('visible');
      });
      return;
    }

    var observer = new IntersectionObserver(function (entries) {
      if (entries[0].isIntersecting && !hasPlayed) {
        hasPlayed = true;

        // Show chaos terminal
        chaosTerminal.classList.add('visible');
        var chaosDuration = animateTerminal(chaosTerminal);

        // Show arrow, then unified terminal
        setTimeout(function () {
          if (arrow) arrow.classList.add('visible');

          setTimeout(function () {
            unifiedTerminal.classList.add('visible');

            setTimeout(function () {
              var unifiedDuration = animateTerminal(unifiedTerminal, function (line) {
                var check = line.querySelector('.t-check');
                if (check) check.classList.add('pop');
              });

              // Animate stats after unified terminal finishes
              setTimeout(animateStats, unifiedDuration + 200);
            }, 250);
          }, 150);
        }, chaosDuration + TERMINAL_GAP);
      }
    }, { threshold: 0.15 });

    observer.observe(container);
  }

  document.addEventListener('DOMContentLoaded', init);
})();
