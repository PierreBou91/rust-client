# milvue_rs

`milvue_rs` is a Rust client library for the Milvue API, providing the ability to submit Digital Imaging and Communications in Medicine (DICOM) files for analysis and fetch the resulting annotations. These annotations can include pathology detection or anatomical measurements.

## Prerequisites

This library is designed to interact with the Milvue API. In order to use this library, you will need an API key and the URL of the Milvue environment you wish to use. Both of these values must be set as environment variables. You can obtain your API key by contacting [Milvue](https://www.milvue.com/).

The source code can be built into a binary executable. Plans are underway to provide binaries for download for all platforms in the near future.

## Dependencies

The `milvue_rs` crate relies on the [dicom-rs](https://github.com/Enet4/dicom-rs) project, a pure Rust implementation of core DICOM standards.

## Features on the Roadmap

Here are some upcoming features in the pipeline:

- Improved logging
- More comprehensive testing
- Continuous Integration (CI) pipeline
- Watcher mode: This feature will allow you to watch a specific folder and automatically send all exams dropped into it for processing.
- Ability to send more than one study at a time for the binary

## Support

If you encounter any issues or have inquiries, you can submit them through Github or email support@milvue.com.
