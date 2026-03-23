(function () {
  'use strict';

  var SPINNER = ['\u280B','\u2819','\u2839','\u2838','\u283C','\u2834','\u2826','\u2827','\u2807','\u280F'];

  var SCENES = [
    {
      id: 'create',
      annotation: 'One command creates addresses for every supported chain',
      actions: [
        { type: 'cmd', text: 'ows wallet create --name agent-treasury' },
        { type: 'pause', ms: 300 },
        { type: 'spin', text: 'Generating wallet...', ms: 700,
          done: '<span class="t-green">\u2713</span> Created wallet <span class="t-bright">agent-treasury</span>' },
        { type: 'pause', ms: 200 },
        { type: 'lines', speed: 0, lines: [''] },
        { type: 'lines', speed: 80, lines: [
          '  <span class="t-dim">Chain              Address                 Path</span>',
          '  <span class="t-dim">\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500</span>',
          '  eip155:1           <span class="t-bright">0xab16...7e3f</span>           m/44\'/60\'/0\'/0/0',
          '  solana:5eykt4U     <span class="t-bright">7Kz9...Bm4x</span>            m/44\'/501\'/0\'/0\'',
          '  bip122:000...e93   <span class="t-bright">bc1q...8k4m</span>            m/84\'/0\'/0\'/0/0',
          '  cosmos:cosmo...    <span class="t-bright">cosmos1...j4kp</span>          m/44\'/118\'/0\'/0/0',
          '  tron:mainnet       <span class="t-bright">TKLm...9xP2</span>            m/44\'/195\'/0\'/0/0',
          '  ton:mainnet        <span class="t-bright">EQCx...Wd3k</span>            m/44\'/607\'/0\'/0\'',
        ]},
      ]
    },
    {
      id: 'sign',
      annotation: 'Every signing request is policy-checked before keys are touched',
      actions: [
        { type: 'cmd', text: 'ows sign tx --wallet agent-treasury --chain evm \\' },
        { type: 'cont', text: '    --to 0x742d35Cc6634C0 --value 0.1ETH' },
        { type: 'pause', ms: 400 },
        { type: 'spin', text: 'Evaluating policies...', ms: 600,
          done: '<span class="t-green">\u2713</span> Policy check passed' },
        { type: 'lines', speed: 60, lines: [
          '  <span class="t-dim">\u251C\u2500</span> Spending limit: 0.1 / 10.0 ETH daily',
          '  <span class="t-dim">\u251C\u2500</span> Address 0x742d...C0 is allowlisted',
          '  <span class="t-dim">\u2514\u2500</span> Chain eip155:1 permitted',
        ]},
        { type: 'pause', ms: 400 },
        { type: 'spin', text: 'Signing...', ms: 500,
          done: '<span class="t-green">\u2713</span> Signed <span class="t-dim">(key decrypted, used, wiped)</span>' },
        { type: 'pause', ms: 200 },
        { type: 'lines', speed: 60, lines: [
          '',
          '  signature: <span class="t-bright">0x3a7f2b...8c9e</span>',
          '  tx hash:   <span class="t-bright">0x8f3a91...2d4b</span>',
        ]},
      ]
    },
    {
      id: 'ecosystem',
      annotation: 'Claude calls tool CLIs that sign via OWS under the hood \u2014 one wallet, every app',
      actions: [
        { type: 'cmd', text: 'ows serve --mcp' },
        { type: 'pause', ms: 200 },
        { type: 'spin', text: 'Starting MCP server...', ms: 400,
          done: '<span class="t-green">\u2713</span> MCP server ready on stdio' },
        { type: 'pause', ms: 600 },
        { type: 'lines', speed: 50, lines: [
          '',
          '<span class="t-blue">[Claude]</span> <span class="t-dim">"Buy \'ETH > $5k\' on Polymarket with agent-treasury"</span>',
          '',
          '<span class="t-dim">\u2192</span> polymarket_buy <span class="t-dim">  \u2190 calls polymarket-cli</span>',
          '  <span class="t-dim">\u2514\u2500 polymarket-cli signs via</span> <span class="t-green">OWS</span> <span class="t-dim">(~/.ows/wallets/agent-treasury)</span>',
        ]},
        { type: 'pause', ms: 200 },
        { type: 'spin', text: 'polymarket-cli \u2192 OWS signing...', ms: 500,
          done: '<span class="t-green">\u2713</span> polymarket-cli \u2192 OWS \u2192 order placed <span class="t-dim">(polygon)</span>' },
        { type: 'pause', ms: 600 },
        { type: 'lines', speed: 50, lines: [
          '',
          '<span class="t-blue">[Claude]</span> <span class="t-dim">"Swap 1 ETH to USDC on Uniswap"</span>',
          '',
          '<span class="t-dim">\u2192</span> uniswap_swap <span class="t-dim">    \u2190 calls uniswap-cli</span>',
          '  <span class="t-dim">\u2514\u2500 uniswap-cli signs via</span> <span class="t-green">OWS</span> <span class="t-dim">(same wallet, no config)</span>',
        ]},
        { type: 'pause', ms: 200 },
        { type: 'spin', text: 'uniswap-cli \u2192 OWS signing...', ms: 500,
          done: '<span class="t-green">\u2713</span> uniswap-cli \u2192 OWS \u2192 swapped <span class="t-dim">(ethereum)</span>' },
        { type: 'pause', ms: 600 },
        { type: 'lines', speed: 50, lines: [
          '',
          '<span class="t-blue">[Claude]</span> <span class="t-dim">"Launch arb-v2 strategy on Dawn"</span>',
          '',
          '<span class="t-dim">\u2192</span> dawn_launch <span class="t-dim">     \u2190 calls dawn-cli</span>',
          '  <span class="t-dim">\u2514\u2500 dawn-cli signs via</span> <span class="t-green">OWS</span> <span class="t-dim">(same wallet, no config)</span>',
        ]},
        { type: 'pause', ms: 200 },
        { type: 'spin', text: 'dawn-cli \u2192 OWS signing...', ms: 500,
          done: '<span class="t-green">\u2713</span> dawn-cli \u2192 OWS \u2192 strategy live <span class="t-dim">(solana)</span>' },
      ]
    },
  ];

  function TerminalDemo(el) {
    this.el = el;
    this.body = el.querySelector('.terminal-body');
    this.annotation = el.querySelector('.terminal-annotation');
    this.tabs = [].slice.call(el.querySelectorAll('.terminal-tab'));
    this.replayBtn = el.querySelector('.terminal-replay');
    this.cancelled = false;
    this.running = false;
    this.currentScene = 0;
    this._tid = null;

    var self = this;

    this.tabs.forEach(function (tab, i) {
      tab.addEventListener('click', function () { self.jumpToScene(i); });
    });

    if (this.replayBtn) {
      this.replayBtn.addEventListener('click', function () { self.restart(); });
    }

    var obs = new IntersectionObserver(function (entries) {
      if (entries[0].isIntersecting && !self.running) {
        self.start();
        obs.disconnect();
      }
    }, { threshold: 0.2 });
    obs.observe(this.el);
  }

  TerminalDemo.prototype.start = function () {
    this.running = true;
    this.cancelled = false;
    this.runAll();
  };

  TerminalDemo.prototype.restart = function () {
    var self = this;
    this.stop();
    setTimeout(function () {
      self.body.innerHTML = '';
      self.currentScene = 0;
      self.setTab(0);
      self.start();
    }, 60);
  };

  TerminalDemo.prototype.jumpToScene = function (i) {
    var self = this;
    this.stop();
    setTimeout(function () {
      self.body.innerHTML = '';
      self.currentScene = i;
      self.setTab(i);
      self.cancelled = false;
      self.running = true;
      self.playScene(SCENES[i]).then(function () {
        if (!self.cancelled) self.showIdle();
      });
    }, 60);
  };

  TerminalDemo.prototype.stop = function () {
    this.cancelled = true;
    this.running = false;
    clearTimeout(this._tid);
  };

  TerminalDemo.prototype.setTab = function (i) {
    this.tabs.forEach(function (t, j) {
      t.classList.toggle('active', j === i);
    });
    if (this.annotation) {
      this.annotation.style.opacity = '0';
      var ann = this.annotation;
      setTimeout(function () {
        ann.textContent = SCENES[i].annotation;
        ann.style.opacity = '1';
      }, 200);
    }
  };

  TerminalDemo.prototype.runAll = function () {
    var self = this;
    var i = 0;

    function nextScene() {
      if (self.cancelled) return;
      if (i >= SCENES.length) {
        i = 0;
        self.body.innerHTML = '';
      }
      self.currentScene = i;
      self.setTab(i);
      if (i > 0) self.body.innerHTML = '';

      self.playScene(SCENES[i]).then(function () {
        if (self.cancelled) return;
        self.showIdle();
        var delay = i < SCENES.length - 1 ? 2500 : 4000;
        i++;
        self._tid = setTimeout(nextScene, delay);
      });
    }

    nextScene();
  };

  TerminalDemo.prototype.playScene = function (scene) {
    var self = this;
    var actions = scene.actions;
    var idx = 0;

    function next() {
      if (self.cancelled || idx >= actions.length) {
        return Promise.resolve();
      }
      var a = actions[idx++];
      var p;
      switch (a.type) {
        case 'cmd':  p = self.typeCmd(a.text); break;
        case 'cont': p = self.typeCont(a.text); break;
        case 'lines': p = self.outLines(a.lines, a.speed); break;
        case 'spin': p = self.doSpin(a.text, a.ms, a.done); break;
        case 'pause': p = self.wait(a.ms); break;
        default: p = Promise.resolve();
      }
      return p.then(next);
    }

    return next();
  };

  TerminalDemo.prototype.typeCmd = function (text) {
    var self = this;
    var line = this.mkLine();
    line.innerHTML = '<span class="t-prompt">$ </span><span class="t-cmd"></span>';
    this.body.appendChild(line);
    var cmd = line.querySelector('.t-cmd');
    var i = 0;

    return new Promise(function (resolve) {
      function tick() {
        if (self.cancelled || i >= text.length) return resolve();
        cmd.textContent += text[i++];
        self.scroll();
        self._tid = setTimeout(tick, 30 + Math.random() * 25);
      }
      tick();
    });
  };

  TerminalDemo.prototype.typeCont = function (text) {
    var self = this;
    var line = this.mkLine();
    line.innerHTML = '<span class="t-cmd"></span>';
    this.body.appendChild(line);
    var cmd = line.querySelector('.t-cmd');
    var i = 0;

    return new Promise(function (resolve) {
      function tick() {
        if (self.cancelled || i >= text.length) return resolve();
        cmd.textContent += text[i++];
        self.scroll();
        self._tid = setTimeout(tick, 25 + Math.random() * 20);
      }
      tick();
    });
  };

  TerminalDemo.prototype.outLines = function (lines, speed) {
    var self = this;
    var i = 0;

    return new Promise(function (resolve) {
      function tick() {
        if (self.cancelled || i >= lines.length) return resolve();
        var line = self.mkLine();
        line.innerHTML = lines[i] || '\u00a0';
        self.body.appendChild(line);
        self.scroll();
        i++;
        if (speed > 0) {
          self._tid = setTimeout(tick, speed);
        } else {
          tick();
        }
      }
      tick();
    });
  };

  TerminalDemo.prototype.doSpin = function (text, ms, doneHtml) {
    var self = this;
    var line = this.mkLine();
    this.body.appendChild(line);
    var start = Date.now();
    var f = 0;

    return new Promise(function (resolve) {
      function tick() {
        if (self.cancelled) return resolve();
        if (Date.now() - start >= ms) {
          line.innerHTML = doneHtml;
          self.scroll();
          return resolve();
        }
        line.innerHTML = '<span class="t-spinner">' + SPINNER[f++ % SPINNER.length] + '</span> <span class="t-dim">' + text + '</span>';
        self.scroll();
        self._tid = setTimeout(tick, 80);
      }
      tick();
    });
  };

  TerminalDemo.prototype.showIdle = function () {
    var line = this.mkLine();
    line.innerHTML = '<span class="t-prompt">$ </span><span class="terminal-cursor"></span>';
    this.body.appendChild(line);
    this.scroll();
  };

  TerminalDemo.prototype.mkLine = function () {
    var d = document.createElement('div');
    d.className = 'terminal-line';
    return d;
  };

  TerminalDemo.prototype.scroll = function () {
    this.body.scrollTop = this.body.scrollHeight;
  };

  TerminalDemo.prototype.wait = function (ms) {
    var self = this;
    return new Promise(function (resolve) {
      self._tid = setTimeout(resolve, ms);
    });
  };

  document.addEventListener('DOMContentLoaded', function () {
    var el = document.getElementById('terminal-demo');
    if (el) new TerminalDemo(el);
  });
})();
