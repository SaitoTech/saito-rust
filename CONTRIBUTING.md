# Contributing

- The next month of sprint planning can always be found on our [github kanban board](https://github.com/orgs/SaitoTech/projects/5)
- PRs should be made against the `main` branch in the [saito-rust project](https://github.com/SaitoTech/saito-rust)
- PRs must pass [rust fmt (via Github Actions)](README.md#github-actions) check and should have full test coverage

### Commit format

We use [Conventional Commits](https://www.conventionalcommits.org) for commit messages.

In short a commit message should follow this format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

For example:

```
feat(keypair): add format check
```

- For the full specification please have a look at https://www.conventionalcommits.org
- Reasons to use this format can be found in this post: https://marcodenisi.dev/en/blog/why-you-should-use-conventional-commits

#### Commit `type`

Most used types:

- `fix`
- `feat`

Further recommended types:

- `build`
- `chore`
- `ci`
- `docs`
- `style`
- `refactor`
- `perf`
- `test`

#### Issue numbers in a commit

If the commit relates to an issue the issue number can be added to the commit-`descrition` or -`body`, i.e.:

```
feat(keypair): add format check #123
```
