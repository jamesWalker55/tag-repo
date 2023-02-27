# tagrepo

A hybrid tag-hierarchy file manager. This is extremely early in development.

## Todo

Database:

- use [slash paths](https://docs.rs/path-slash/latest/path_slash/) for paths
- ensure tags are alphabetically ordered

Query

- allow querying based on parent folder path
- make sure querying still works without the `#all` token

Frontend

- study virtualized list implementations then build your own
- 2 views: row / gallery

Code quality

- optimise functions that use strings as input:
  - hint: `AsRef<str>` is not the best choice
  - https://hermanradtke.com/2015/05/06/creating-a-rust-function-that-accepts-string-or-str.html/
  - https://hermanradtke.com/2015/05/29/creating-a-rust-function-that-returns-string-or-str.html/
  - https://lise-henry.github.io/articles/optimising_strings.html
