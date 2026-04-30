Generated flutter_rust_bridge files will be written here.

Expected files after running codegen:

- `api.dart`
- `bridge_generated.dart`
- any platform-specific loader files required by the selected generator version

`api.dart` is currently a placeholder so Flutter analyze/test can run before
codegen. `scripts/generate-flutter-bridge.sh` temporarily removes the
placeholder before running `flutter_rust_bridge_codegen generate`, then restores
it if the generator version does not emit `api.dart` at this path.

Do not hand-edit generated Dart files beyond placeholder maintenance.
