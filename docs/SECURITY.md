# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.4.x   | ✅ Yes             |
| < 0.4.x | ❌ No              |

## Reporting a Vulnerability

If you discover a security vulnerability in Axiom Core, please follow these steps:

### 1. Private Disclosure

**Do not open a public issue** on GitHub. Instead, contact the security team via:

- **Email**: security@axiom-core.dev
- **PGP**: Available upon request

### 2. What to Include

Please provide the following information in your report:

- Description of the vulnerability
- Steps to reproduce
- Impact assessment
- Any potential mitigations or fixes
- Your contact information for follow-up

### 3. Response Timeline

We aim to respond to security reports within **48 hours**. The response process:

1. **Acknowledgment**: Confirmation of receipt within 48 hours
2. **Assessment**: Initial severity assessment within 5 business days
3. **Fix**: Development of a fix within 14 days for critical issues
4. **Release**: Patch release within 30 days
5. **Disclosure**: Public announcement after the fix is released

## Security Practices

### Authentication

- All API endpoints require JWT authentication
- Multi-factor authentication is recommended for production deployments
- API keys should be rotated every 90 days

### Authorization

- Four-level permission system (read, write, admin, super)
- Default-deny policy: no permissions granted by default
- Role-based access control (RBAC) for multi-user deployments

### Data Protection

- SQLite database uses WAL mode for atomic writes
- Regular backups are performed automatically
- Sensitive data (API keys, tokens) should be stored in environment variables or secret managers
- TLS 1.3 required for all external communication

### Input Validation

- All user inputs are validated before processing
- Sanitization for SQL injection prevention
- Rate limiting to prevent brute-force attacks

### Logging & Monitoring

- Structured JSON logging enabled by default
- Distributed tracing with OpenTelemetry
- Prometheus metrics for anomaly detection
- Alerting for security-related events

### Dependency Security

- Regular `cargo audit` scans
- Security audit integration in CI pipeline
- Dependencies are pinned to specific versions
- Vulnerability notifications are monitored

## Security Features

### Built-in Security Measures

1. **Entropy Governance**: Automatic detection and mitigation of anomalous behavior
2. **Supervision Tree**: Automatic restart and recovery of failed components
3. **Circuit Breakers**: Protection against cascading failures
4. **Rate Limiting**: Protection against abuse
5. **JWT Authentication**: Stateless token-based authentication

### Security Hardening

- No root access required for runtime operations
- Isolated process execution for plugins
- Network access control via MCP security layer
- Environment-based configuration for sensitive settings

## Incident Response

### Detection

- Automated monitoring for suspicious patterns
- Alerting via configured channels
- Regular security log reviews

### Response

1. **Contain**: Isolate affected components
2. **Eradicate**: Remove the vulnerability
3. **Recover**: Restore normal operations
4. **Learn**: Update security policies and procedures

### Post-Incident Review

- Documentation of the incident
- Root cause analysis
- Updates to security measures
- Training if applicable

## Compliance

### Standards

- OWASP Top 10 compliance
- ISO 27001 controls where applicable
- GDPR compliance for data handling

### Audit Trail

- Complete audit log of all security-relevant events
- Immutable event storage via SQLite WAL
- Regular security audits recommended

## Contact

For security-related questions or reports:

- **Email**: security@axiom-core.dev
- **GitHub**: [Security Advisories](https://github.com/arwei944/axiom-kernel/security/advisories)

---

*Last updated: 2026-07-05*
