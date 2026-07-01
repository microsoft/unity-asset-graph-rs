# unity-asset-graph-rs

Build a database from a Unity project's assets and packages, and query it.

## Usage

Before 
```bash
$ cargo run -- --help
Usage: unity-asset-graph-rs.exe [OPTIONS] <COMMAND>

Commands:
  find-assets       Find assets in a Unity project directory and create a database file
  resolve-assets    Scan all assets in the database to identify their dependencies
  info              Get information about a specific asset by ID or name
  find-unused       Find unused assets in the database
  find-broken-refs  Find broken references in the database
  help              Print this message or the help of the given subcommand(s)

Options:
  -d, --db-path <DB_PATH>  Path to the database file (default: db.bin) [default: db.bin]
  -h, --help               Print help
```

## Building

To build the project, you'll need a [Rust compiler](https://www.rust-lang.org/tools/install) installed.

```bash
cargo build --release
```

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit [Contributor License Agreements](https://cla.opensource.microsoft.com).

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.
