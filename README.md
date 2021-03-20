# Quanto Falta?

This software computes how much time it will take to vaccinate enough of Brazil's population
(it's currently hardcoded, but you can easily change it to use other country) taking into
account the current vaccination rate (per Our World in Data).

It tweets the estimate and also generates a HTML page.

Using GitHub Actions, it's called daily to tweet at https://twitter.com/quantofaltacov
and to update https://quantofalta.github.io/

It's written in Rust, which is really overengineering for such a simple tool, but it helped
me learn it.
