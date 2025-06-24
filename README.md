[![progress-banner](https://backend.codecrafters.io/progress/git/9cf7daf5-5dff-48b4-9ebc-8926e782689b)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

This is a starting point for Rust solutions to the
["Build Your Own Git" Challenge](https://codecrafters.io/challenges/git).

In this challenge, you'll build a small Git implementation that's capable of
initializing a repository, creating commits and cloning a public repository.
Along the way we'll learn about the `.git` directory, Git objects (blobs,
commits, trees etc.), Git's transfer protocols and more.

**Note**: If you're viewing this repo on GitHub, head over to
[codecrafters.io](https://codecrafters.io) to try the challenge.



<h1 align="center">Git from scratch in Rust</h1>

<div align="center">
    <img src="/rust-image-3.png" alt="Project progress image">
</div>

### Stages:
1. Initialize the `.git` repository - `git init`
2. Read a `blob` object - `git cat-file`
3. Create a `blob` object - `git hash-object`
4. Read a `tree` object - `git ls-tree`
5. Write a `tree` object - `git write-tree`
6. Create a `commit` - `git commit-tree`
7. `Clone` repository