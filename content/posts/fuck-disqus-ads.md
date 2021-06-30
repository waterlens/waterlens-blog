---
title: "Fuck Disqus Ads"
date: 2021-06-30T11:19:29+08:00
---

The Disqus never give up its efforts to pollute my blog with trolling ads. I mean, if they are intent on throwing rubbish at users, they had better choose those ads more appropriately.

In the previous commits, I use simple CSS to block the ads. But today I just found it failed. The Disqus changed their `<iframe>` for showing disgusting ads and no `src` attribute anymore.

That's nothing.

Just change the additional CSS from

<pre><code class="language-css match-braces rainbow-braces">
iframe[src*=ads-iframe] {
  display: none !important;
}
</code></pre>

to this one

<pre><code class="language-css match-braces rainbow-braces">
  iframe[title*=Disqus][sandbox*=allow] {
    display: none !important;
  }
</code></pre>

Have a nice profit, Disqus!
