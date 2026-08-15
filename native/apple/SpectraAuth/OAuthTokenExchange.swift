import Foundation

public enum SpectraOAuthTokenExchangeError: Error {
    case unsupportedProvider
    case missingTokenEndpoint
    case invalidTokenEndpoint
    case invalidResponse
    case providerRejected
    case vaultFailed
}

public struct SpectraOAuthTokenResponse: Codable, Sendable {
    public let accessToken: String
    public let refreshToken: String?
    public let expiresIn: Int?

    private enum CodingKeys: String, CodingKey {
        case accessToken = "access_token"
        case refreshToken = "refresh_token"
        case expiresIn = "expires_in"
    }
}

public final class SpectraOAuthTokenExchange {
    private let session: URLSession

    public init(session: URLSession = .shared) {
        self.session = session
    }

    public func exchange(
        providerId: String,
        clientId: String,
        code: String,
        codeVerifier: String,
        redirectURI: String,
        clientSecret: String? = nil,
        projectId: String? = nil,
        vault: SpectraKeychainCredentialVault
    ) async throws -> SpectraStoredCredential {
        guard let configuration = SpectraOAuthProviderConfiguration.forProvider(providerId, projectId: projectId) else {
            throw SpectraOAuthTokenExchangeError.unsupportedProvider
        }
        guard let tokenURL = configuration.tokenURL else {
            throw SpectraOAuthTokenExchangeError.missingTokenEndpoint
        }
        guard tokenURL.scheme?.lowercased() == "https",
              ["api.openai.com", "api.anthropic.com"].contains(tokenURL.host?.lowercased()) else {
            throw SpectraOAuthTokenExchangeError.invalidTokenEndpoint
        }

        var components = URLComponents()
        components.queryItems = [
            URLQueryItem(name: "grant_type", value: "authorization_code"),
            URLQueryItem(name: "client_id", value: clientId),
            URLQueryItem(name: "code", value: code),
            URLQueryItem(name: "redirect_uri", value: redirectURI),
            URLQueryItem(name: "code_verifier", value: codeVerifier),
            URLQueryItem(name: "scope", value: configuration.scopes)
        ]
        if let clientSecret, !clientSecret.isEmpty {
            components.queryItems?.append(URLQueryItem(name: "client_secret", value: clientSecret))
        }

        var request = URLRequest(url: tokenURL)
        request.httpMethod = "POST"
        request.timeoutInterval = 30
        request.setValue("application/x-www-form-urlencoded", forHTTPHeaderField: "Content-Type")
        request.httpBody = components.percentEncodedQuery?.data(using: .utf8)

        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw SpectraOAuthTokenExchangeError.invalidResponse
        }
        guard (200..<300).contains(httpResponse.statusCode) else {
            throw SpectraOAuthTokenExchangeError.providerRejected
        }

        let token: SpectraOAuthTokenResponse
        do {
            token = try JSONDecoder().decode(SpectraOAuthTokenResponse.self, from: data)
        } catch {
            throw SpectraOAuthTokenExchangeError.invalidResponse
        }
        guard !token.accessToken.isEmpty else {
            throw SpectraOAuthTokenExchangeError.invalidResponse
        }

        let expiresAt = token.expiresIn.map { String(Int(Date().timeIntervalSince1970) + $0) }
        let credential = SpectraStoredCredential(
            providerId: providerId,
            accessToken: token.accessToken,
            refreshToken: token.refreshToken,
            expiresAt: expiresAt
        )
        do {
            try vault.write(credential)
        } catch {
            throw SpectraOAuthTokenExchangeError.vaultFailed
        }
        return credential
    }
}
