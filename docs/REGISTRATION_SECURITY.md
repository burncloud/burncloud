# Registration System Security & Rate Limiting

## Overview
This document outlines the security measures implemented in the registration system and recommendations for additional backend protections.

## Implemented Security Features

### 1. Input Sanitization (Client-Side)
- **Location**: `crates/client/crates/client-shared/src/utils/validation.rs`
- **Implementation**: `sanitize_input()` function
- **Protection**: XSS prevention by escaping HTML special characters
- **Characters escaped**: `<`, `>`, `"`, `'`, `&`

```rust
pub fn sanitize_input(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
```

### 2. Password Security
- **Hashing**: bcrypt with DEFAULT_COST (12 rounds)
- **No Logging**: Passwords are never logged in frontend or backend
- **Strength Validation**: Client-side password strength meter guides users to create secure passwords
- **Minimum Requirements**: 6 characters minimum (should be increased to 8+ in production)

### 3. Username Validation
- **Format**: 3-20 characters, alphanumeric and underscore only
- **Regex**: `^[a-zA-Z0-9_]{3,20}$`
- **Availability Check**: Async endpoint to check if username is taken before submission

### 4. Email Validation
- **Format**: RFC 5322-compliant email validation
- **Optional**: Email is not required for registration
- **Real-time**: Debounced validation provides immediate feedback

## Required Backend Implementations

### 1. Rate Limiting
**Priority**: HIGH

Rate limiting should be implemented at multiple levels:

#### A. IP-Based Rate Limiting
```
Recommended limits for /console/api/user/register:
- 5 registrations per IP per hour
- 20 registrations per IP per day
```

#### B. Username Check Rate Limiting
```
Recommended limits for /console/api/user/check_username:
- 30 requests per IP per minute
- Implement caching for repeated username checks
```

#### C. Login Attempt Rate Limiting
```
Recommended limits for /console/api/user/login:
- 5 failed attempts per username per 15 minutes
- 10 failed attempts per IP per hour
- Exponential backoff after failed attempts
```

**Implementation Suggestions**:
- Use Redis for distributed rate limiting
- Consider implementing with middleware like `tower-governor` or `tower-http`
- Log rate limit violations for security monitoring

### 2. CSRF Protection
**Priority**: HIGH

**Current Status**: Not implemented

**Recommendations**:
1. Implement CSRF tokens for state-changing operations
2. Use SameSite cookie attribute
3. Validate Origin/Referer headers
4. Consider using axum-csrf crate

Example implementation:
```rust
// Add to your middleware
use axum_csrf::{CsrfConfig, CsrfLayer};

let csrf_config = CsrfConfig::default();
let app = Router::new()
    .layer(CsrfLayer::new(csrf_config));
```

### 3. Input Validation (Server-Side)
**Priority**: HIGH

While client-side validation exists, **server-side validation is critical**:

```rust
// Example server-side validation
fn validate_username(username: &str) -> Result<(), String> {
    if username.len() < 3 || username.len() > 20 {
        return Err("Username must be 3-20 characters".to_string());
    }
    
    let username_regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
    if !username_regex.is_match(username) {
        return Err("Username can only contain letters, numbers, and underscores".to_string());
    }
    
    Ok(())
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }
    
    // Add additional complexity requirements
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    
    if !(has_uppercase && has_lowercase && has_digit) {
        return Err("Password must contain uppercase, lowercase, and digit".to_string());
    }
    
    Ok(())
}
```

### 4. CAPTCHA Integration
**Priority**: MEDIUM

For production environments, consider adding CAPTCHA:
- Google reCAPTCHA v3 (invisible, score-based)
- hCaptcha (privacy-focused alternative)
- Cloudflare Turnstile

Implement on:
- Registration form (after 3 failed attempts)
- Login form (after 2 failed attempts)
- Username availability checks (if abuse detected)

### 5. Email Verification
**Priority**: MEDIUM (if email is collected)

**Implementation Plan**:
1. Send verification email upon registration
2. Store verification token in database
3. Require email verification before full account access
4. Implement token expiration (24 hours)
5. Provide resend verification option

### 6. Account Lockout
**Priority**: MEDIUM

Implement temporary account lockout after repeated failed login attempts:
```
- 5 failed attempts: 15-minute lockout
- 10 failed attempts: 1-hour lockout
- 15 failed attempts: 24-hour lockout or manual review
```

### 7. Security Headers
**Priority**: HIGH

Ensure these HTTP security headers are set:
```
Content-Security-Policy: default-src 'self'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
Strict-Transport-Security: max-age=31536000; includeSubDomains
Referrer-Policy: strict-origin-when-cross-origin
```

## Monitoring & Logging

### Events to Log (Security)
1. Failed registration attempts (with reason)
2. Successful registrations (with timestamp, IP)
3. Rate limit violations
4. Suspicious patterns (rapid requests, etc.)
5. Failed login attempts

### DO NOT Log
1. Passwords (plaintext or hashed)
2. Email addresses in plain error messages
3. Full IP addresses in public-facing logs (consider GDPR)
4. Session tokens or authentication credentials

## Testing Security

### Unit Tests
- Input validation edge cases
- XSS payload sanitization
- Password hashing verification

### Integration Tests
- Rate limiting enforcement
- CSRF token validation
- Email verification flow

### Security Scanning
- Regular dependency audits: `cargo audit`
- OWASP Top 10 compliance checks
- Penetration testing before production

## Compliance Considerations

### GDPR (if serving EU users)
- Obtain consent for data collection
- Provide data export functionality
- Implement right to deletion
- Maintain audit logs

### CCPA (if serving California users)
- Provide privacy notice
- Allow opt-out of data sale
- Respond to data access requests

## Production Checklist

- [ ] Implement server-side rate limiting
- [ ] Add CSRF protection
- [ ] Enable all security headers
- [ ] Implement email verification
- [ ] Add account lockout mechanism
- [ ] Set up security monitoring/alerting
- [ ] Configure CAPTCHA for high-risk actions
- [ ] Perform security audit
- [ ] Document incident response procedures
- [ ] Train team on security best practices

## References

- [OWASP Registration Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Registration_Cheat_Sheet.html)
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
