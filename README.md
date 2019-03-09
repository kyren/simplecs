## A demonstration of an alternative ECS API style ##

**Don't use this!** It was uploaded as a demonstration of an alternate API style
for ECS systems that focuses on just the "EC" part of ECS: just a fancy
queryable data structure.

However, as it is written, this is pretty slow in comparison to something like
'specs', and by dropping the "S" portion of "ECS", of course it doesn't contain
things like a system scheduler which can be important for performance reasons.

I won't really be maintaining this, but it may be useful as a design reference.
Probably the most interesting parts are the "multi-lock" code and the basic idea
of expressing ECS as "just a data structure".  Look at
[src/tests/world.rs](src/tests/world.rs) for a usage example.

It would be great if somebody could take this idea and make something fast out
of it for when other ECS libraries might feel too restrictive, or just to
further experiment.

## License ##

This code is licensed under either of:

* MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
* Creative Commons CC0 1.0 Universal Public Domain Dedication
  [LICENSE-CC0](LICENSE-CC0) or
  https://creativecommons.org/publicdomain/zero/1.0/

at your option.
