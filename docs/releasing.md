# Releasing

1. Give the version its heading in `CHANGELOG.md`, in place of `Unreleased`. Set the version in `Cargo.toml`,
   `editors/vscode/package.json` and `js/package.json`, and in `php/laravel/composer.json` the `hypra/runtime`
   version it requires and the version its path repository gives `php/runtime`. Run `cargo build` and
   `npm install` in `editors/vscode` to update the lock files, and commit.
2. Tag it and push the tag:

   ```console
   $ git tag v0.2.0
   $ git push origin v0.2.0
   ```

   The `release` workflow checks that the versions match the tag, builds the compiler for Linux, macOS and
   Windows (x64 and ARM) and the VS Code extension, and publishes them as a GitHub release with `SHA256SUMS`.
   `php artisan hypra:install` downloads from there. Then the `registries` workflow publishes the compiler on
   crates.io and the `hypra` package, with the compilers from the release in it, on npm, both through trusted
   publishing, and the extension on Open VSX, with the `OVSX_PAT` secret.
3. Publish the PHP packages. Packagist needs each in its own repository, so `php/runtime` and `php/laravel` are
   split into `hypralang/hypra-runtime` and `hypralang/hypra-laravel` and tagged with the same version:

   ```console
   $ git subtree split --prefix php/runtime -b split-runtime
   $ git push git@github.com:hypralang/hypra-runtime.git split-runtime:main
   $ git push git@github.com:hypralang/hypra-runtime.git split-runtime:refs/tags/v0.2.0
   ```

   and the same for `php/laravel`. Packagist picks up new tags by itself once the repositories are registered.
4. Publish the extension on the Visual Studio Marketplace: `cd editors/vscode && npx vsce publish`.

## The website

The `site` workflow builds `site/` on every pull request and publishes it to hypralang.dev with GitHub Pages on
every push to `main`.

## The first npm release

npm trusts a workflow only for a package that exists. Before the first release that publishes to npm, create an
access token with permission to publish on npmjs.com and save it as the `NPM_TOKEN` secret of the repository;
the `npm` job uses it once. After that release, add `hypralang/hypra` with the workflow `registries.yml` as a
trusted publisher in the package's settings on npmjs.com, and delete the token and the secret.
