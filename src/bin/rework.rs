#[tokio::main]
async fn main() {
    // Get folder path from args
    // Parse folder recursively to get all dicom files
    // ?? Regroup dicom files by study instance UID ??
    // Spawn a thread to send each study to Milvue
    // Get output folder from args, defaults to .
    // Create output folder if it doesn't exist
}
