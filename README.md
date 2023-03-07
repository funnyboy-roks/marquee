# Marquee

[![Crates.io](https://img.shields.io/crates/v/marquee.svg)](https://crates.io/crates/marquee)

This is a CLI tool that will convert each line of stdin into a "marquee"
style output.  An example can be seen below.

![GIF of marquee usage](./img/usage.gif)

## Install

Install using Cargo:

```sh
cargo install marqee
```

## Usage

```sh
echo "Hello World" | marquee
```

See `marquee --help` for advanced usage.

### `--json`

If specifying the `--json` flag, the json values are as follows:

```jsonc
{
    "content": "required string", // The content of the string that will be rotating
    "prefix": "optional string",  // The prefix before the string
    "suffix": "optional string",  // The suffix after the string
    "rotate": "optional boolean"  // If the string should rotate (default: true)
}
```

_Note: If specifying both `--prefix` and `prefix` in the JSON or
`--suffix` and `suffix` in the JSON, then the output will take the form
of `{global_prefix}{prefix}{content}{suffix}{global_suffix}`_


## Todo

Some of the todo items that I have in mind (feel free to create issues
for more or PRs to implement these)

- [ ] Better documentation
- [ ] Make JSON a feature rather than enabled by default
    - This would probably keep the `--json` arg, but give a useful
    message, similar to how `exa` does with their `--git` arg.
    - Ideally it would remove serde from the build all together
- [ ] Convert all `unwrap`/`expect` to properly handle errors
- [ ] Add more features to the JSON input (I'm not 100% on these)
    - `speed: u64` - The speed at which the message should rotate
    - `parts: &[String]` - Parts that should rotate separately, this
    would require quite a large rewrite to be done well, I think
- [ ] More CLI configuration

