import Foundation

public struct SpectraOAuthProviderConfiguration: Sendable {
    public let providerId: String
    public let authorizeURL: URL?
    public let tokenURL: URL?
    public let scopes: String
    public let requiresClientId: Bool
    public let quotaEndpoint: URL?
    public let quotaEndpointStatus: String

    public init(
        providerId: String,
        authorizeURL: URL?,
        tokenURL: URL?,
        scopes: String,
        requiresClientId: Bool,
        quotaEndpoint: URL?,
        quotaEndpointStatus: String
    ) {
        self.providerId = providerId
        self.authorizeURL = authorizeURL
        self.tokenURL = tokenURL
        self.scopes = scopes
        self.requiresClientId = requiresClientId
        self.quotaEndpoint = quotaEndpoint
        self.quotaEndpointStatus = quotaEndpointStatus
    }

    public static func forProvider(_ providerId: String, projectId: String? = nil) -> Self? {
        switch providerId {
        case "codex":
            return Self(
                providerId: providerId,
                authorizeURL: nil,
                tokenURL: nil,
                scopes: "",
                requiresClientId: false,
                quotaEndpoint: URL(string: "https://api.openai.com/v1/organization/usage/completions"),
                quotaEndpointStatus: "organization-usage-only"
            )
        case "claude":
            return Self(
                providerId: providerId,
                authorizeURL: nil,
                tokenURL: nil,
                scopes: "",
                requiresClientId: false,
                quotaEndpoint: URL(string: "https://api.anthropic.com/v1/organizations/usage_report/messages"),
                quotaEndpointStatus: "organization-usage-only"
            )
        default:
            return nil
        }
    }
}
