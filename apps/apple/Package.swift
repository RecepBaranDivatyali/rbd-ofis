// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RbdOfis",
    defaultLocalization: "tr",
    platforms: [
        .macOS(.v13),
        .iOS(.v16)
    ],
    products: [
        .library(
            name: "RbdOfis",
            targets: ["RbdOfis"]
        ),
    ],
    targets: [
        .target(
            name: "CRbdFfi",
            dependencies: [],
            path: "Sources/CRbdFfi",
            publicHeadersPath: "include"
        ),
        .target(
            name: "RbdOfis",
            dependencies: ["CRbdFfi"],
            path: "Sources/RbdOfis"
        ),
        .testTarget(
            name: "RbdOfisTests",
            dependencies: ["RbdOfis"],
            path: "Tests/RbdOfisTests"
        ),
    ]
)
