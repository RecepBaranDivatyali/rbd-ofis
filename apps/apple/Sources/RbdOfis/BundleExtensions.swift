// BundleExtensions.swift - RBD OFİS Apple Bundle Access
// AGPL-3.0-or-later

import Foundation

#if !SWIFT_PACKAGE
private class BundleFinder {}
extension Foundation.Bundle {
    internal static var module: Bundle {
        return Bundle(for: BundleFinder.self)
    }
}
#endif
