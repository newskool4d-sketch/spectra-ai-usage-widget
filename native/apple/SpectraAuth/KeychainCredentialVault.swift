import Foundation
import Security

public struct SpectraStoredCredential: Codable, Sendable {
    public let providerId: String
    public let accessToken: String
    public let refreshToken: String?
    public let expiresAt: String?

    public init(providerId: String, accessToken: String, refreshToken: String? = nil, expiresAt: String? = nil) {
        self.providerId = providerId
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.expiresAt = expiresAt
    }
}

public enum SpectraCredentialVaultError: Error {
    case invalidProvider
    case invalidCredential
    case encodingFailed
    case decodingFailed
    case keychain(OSStatus)
}

public final class SpectraKeychainCredentialVault {
    public static let defaultService = "com.spectra.ai-usage-widget"

    private let service: String

    public init(service: String = SpectraKeychainCredentialVault.defaultService) {
        self.service = service
    }

    private func account(for providerId: String) throws -> String {
        let valid = providerId.range(of: "^[a-z0-9-]+$", options: .regularExpression) != nil
        guard valid else { throw SpectraCredentialVaultError.invalidProvider }
        return "provider:\(providerId)"
    }

    private func baseQuery(providerId: String) throws -> [CFString: Any] {
        let account = try account(for: providerId)
        return [
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: service,
            kSecAttrAccount: account
        ]
    }

    public func read(providerId: String) throws -> SpectraStoredCredential? {
        var query = try baseQuery(providerId: providerId)
        query[kSecReturnData] = kCFBooleanTrue
        query[kSecMatchLimit] = kSecMatchLimitOne

        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status != errSecItemNotFound else { return nil }
        guard status == errSecSuccess else { throw SpectraCredentialVaultError.keychain(status) }
        guard let data = result as? Data else { throw SpectraCredentialVaultError.decodingFailed }

        do {
            let credential = try JSONDecoder().decode(SpectraStoredCredential.self, from: data)
            guard credential.providerId == providerId else {
                throw SpectraCredentialVaultError.decodingFailed
            }
            return credential
        } catch let error as SpectraCredentialVaultError {
            throw error
        } catch {
            throw SpectraCredentialVaultError.decodingFailed
        }
    }

    public func write(_ credential: SpectraStoredCredential) throws {
        guard !credential.accessToken.isEmpty else {
            throw SpectraCredentialVaultError.invalidCredential
        }
        _ = try account(for: credential.providerId)
        guard let data = try? JSONEncoder().encode(credential) else {
            throw SpectraCredentialVaultError.encodingFailed
        }

        let query = try baseQuery(providerId: credential.providerId)
        let attributes: [CFString: Any] = [
            kSecValueData: data,
            kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        guard updateStatus == errSecItemNotFound else {
            guard updateStatus == errSecSuccess else { throw SpectraCredentialVaultError.keychain(updateStatus) }
            return
        }

        var addQuery = query
        attributes.forEach { addQuery[$0.key] = $0.value }
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        guard addStatus == errSecSuccess else { throw SpectraCredentialVaultError.keychain(addStatus) }
    }

    public func remove(providerId: String) throws {
        let query = try baseQuery(providerId: providerId)
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw SpectraCredentialVaultError.keychain(status)
        }
    }
}
