# ridɛ
Small "Rust IDE" demo I put together for my [senior project](assets/atestat.pdf) ("atestat" in Romanian) at the
["Tudor Vianu" High School of Computer Science](https://en.wikipedia.org/wiki/Tudor_Vianu_National_College_of_Computer_Science).
The git tag `atestat-cnitv-2015` is almost identical to the submitted code.

It mostly serves to show that driving the Rust compiler for the purposes of an IDE is possible, and it's
pretty limited in scope, displaying types of expressions in compiling programs being significantly easier
than autocompleting partially valid expressions, for example.

It should be possible to extract the right data from cargo to handle multi-file projects and then, by only
performing intra-function type-checking where necessary, even large projects could be quickly reanalyzed.

Suffice to say, both the IDE and the UI sides of it have a long way to go.
No promises on future development, just that something interesting may happen here.
