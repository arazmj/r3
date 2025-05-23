# R3 - S3 Compatible Storage in Rust

R3 is an open-source, S3-compatible storage service written in Rust. Designed for robustness, performance, and scalability, R3 offers a reliable storage solution for various applications, providing seamless integration with existing S3 clients and tools.

## Features

### Authentication
- User registration and login
- Secure password storage
- Session management

### Bucket Management
- Create buckets
- Delete buckets
- Read bucket information
- Update bucket settings
- Bucket policies management
  - Set bucket policies
  - Get bucket policies

### Object Management
- Upload objects
- Download objects
- Delete objects
- Read object metadata

### Multipart Upload
- Initiate multipart uploads
- Upload parts
- Complete multipart uploads
- Abort multipart uploads
- Part management with ETags

### Versioning
- Enable/disable bucket versioning
- List object versions
- Get specific object versions
- Delete object versions

### Technical Features
- **S3 Compatibility**: Full support for the S3 API, enabling easy integration with existing S3 clients
- **High Performance**: Built with Rust for optimal performance and safety
- **RESTful API**: Clean and intuitive API design
- **Error Handling**: Comprehensive error handling and reporting
- **Unit Tests**: Extensive test coverage for all features
- **Modular Design**: Clean architecture with separate modules for different functionalities

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) 1.56.0 or higher
- [Docker](https://www.docker.com/) (optional, for containerized deployment)

### Installation

1. Clone the repository:
   ```sh
   git clone https://github.com/yourusername/r3.git
   cd r3
   ```

2. Build the project:
   ```sh
   cargo build --release
   ```

3. Run the server:
   ```sh
   cargo run --release
   ```

The server will start on `http://localhost:8080` by default.

### API Usage

#### Authentication
```http
POST /register
Content-Type: application/json

{
    "username": "your_username",
    "password": "your_password"
}
```

```http
POST /login
Content-Type: application/json

{
    "username": "your_username",
    "password": "your_password"
}
```

#### Bucket Operations
```http
POST /{bucket}  # Create bucket
DELETE /{bucket}  # Delete bucket
GET /{bucket}  # Get bucket info
PUT /{bucket}  # Update bucket
```

#### Object Operations
```http
PUT /{bucket}/{key}  # Upload object
GET /{bucket}/{key}  # Download object
DELETE /{bucket}/{key}  # Delete object
```

#### Multipart Upload
```http
POST /{bucket}/{key}?uploads  # Initiate multipart upload
PUT /{bucket}/{key}?uploadId={uploadId}&partNumber={partNumber}  # Upload part
POST /{bucket}/{key}?uploadId={uploadId}  # Complete multipart upload
DELETE /{bucket}/{key}?uploadId={uploadId}  # Abort multipart upload
```

#### Versioning
```http
PUT /{bucket}?versioning  # Enable/disable versioning
GET /{bucket}?versions  # List object versions
GET /{bucket}/{key}?versionId={versionId}  # Get specific version
DELETE /{bucket}/{key}?versionId={versionId}  # Delete specific version
```

## Development

### Running Tests
```sh
cargo test
```

### Code Style
The project follows Rust's standard formatting guidelines. To format your code:
```sh
cargo fmt
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.