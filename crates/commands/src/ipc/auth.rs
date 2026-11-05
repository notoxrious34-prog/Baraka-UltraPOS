//! Authentication and operator-management commands.
use super::{error_code, parse_wire_id, require_session, sorted_permission_codes, AppState};
use baraka_application::errors::ApplicationError;
use baraka_application::ports::SessionToken;
use baraka_application::use_cases::users::{
    CreateUserUseCase, ListUsersUseCase, ResetUserCredentialsUseCase, UpdateUserUseCase,
};
use baraka_application::use_cases::{
    AuthenticateUserUseCase, BootstrapFirstAdminInput, BootstrapFirstAdminUseCase, LogoutUseCase,
};
use baraka_domain::role::Role;

use crate::dto::{
    BootstrapFirstAdminRequest, BootstrapFirstAdminResponse, CheckFirstRunStatusResponse,
    CreateUserRequestDto, LoginRequestDto, LoginResponseDto, LogoutRequestDto,
    ResetUserCredentialsRequestDto, UpdateUserRequestDto, UserSummaryDto,
};

/// `auth_login` command: username + password authentication.
#[tauri::command(rename_all = "snake_case")]
pub fn auth_login(
    state: tauri::State<'_, AppState>,
    request: LoginRequestDto,
) -> Result<LoginResponseDto, String> {
    let login = AuthenticateUserUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.sessions.as_ref(),
        state.clock.as_ref(),
    );
    match login.execute(&request.username, &request.password) {
        Ok(session) => Ok(LoginResponseDto {
            session_token: session.token.as_str().to_string(),
            user_id: session.user_id.to_string(),
            username: session.username,
            role: session.role.code().to_string(),
            permissions: sorted_permission_codes(session.role),
            expires_at: session.expires_at.to_rfc3339(),
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `auth_logout` command: revoke a session (idempotent).
#[tauri::command(rename_all = "snake_case")]
pub fn auth_logout(
    state: tauri::State<'_, AppState>,
    request: LogoutRequestDto,
) -> Result<(), String> {
    let logout = LogoutUseCase::new(state.sessions.as_ref());
    let token = SessionToken(request.session_token);
    match logout.execute(&token) {
        Ok(()) => Ok(()),
        Err(error) => Err(error_code(&error)),
    }
}

/// `check_first_run_status` command: fast read-only first-run probe.
///
/// Unauthenticated by definition (no session exists yet on a fresh
/// database). Safe to call repeatedly; carries zero side effects.
#[tauri::command(rename_all = "snake_case")]
pub fn check_first_run_status(
    state: tauri::State<'_, AppState>,
) -> Result<CheckFirstRunStatusResponse, String> {
    match state.users.count_users() {
        Ok(count) => Ok(CheckFirstRunStatusResponse {
            requires_setup: count == 0,
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `bootstrap_first_admin` command: one-time first Admin creation.
///
/// Unauthenticated by definition. The repository rejects the call with
/// `BOOTSTRAP_ALREADY_COMPLETED` once any user exists; the frontend then
/// logs in through the standard `auth_login` flow (no session token is
/// minted here). `origin_device_id` comes from TerminalContext-held state
/// (ADR-045), never from client input.
#[tauri::command(rename_all = "snake_case")]
pub fn bootstrap_first_admin(
    state: tauri::State<'_, AppState>,
    request: BootstrapFirstAdminRequest,
) -> Result<BootstrapFirstAdminResponse, String> {
    let bootstrap = BootstrapFirstAdminUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    );
    match bootstrap.execute(
        &BootstrapFirstAdminInput {
            username: request.username,
            password: request.password,
        },
        &state.terminal_id,
    ) {
        Ok(output) => Ok(BootstrapFirstAdminResponse {
            user_id: output.user_id.to_string(),
            username: output.username,
        }),
        Err(error) => Err(error_code(&error)),
    }
}

/// `users_list` command: every managed operator (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_list(
    state: tauri::State<'_, AppState>,
    session_token: String,
) -> Result<Vec<UserSummaryDto>, String> {
    let session = require_session(&state, &session_token)?;
    let records = ListUsersUseCase::new(state.users.as_ref())
        .execute(&session)
        .map_err(|error| error_code(&error))?;
    Ok(records.iter().map(to_user_summary).collect())
}

/// `users_create` command: validated, hashed operator creation (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_create(
    state: tauri::State<'_, AppState>,
    request: CreateUserRequestDto,
) -> Result<UserSummaryDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let role = Role::from_code(request.role.trim())
        .map_err(|error| error_code(&ApplicationError::Validation(error)))?;
    let record = CreateUserUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        &request.username,
        &request.password,
        request.pin.as_deref(),
        role,
    )
    .map_err(|error| error_code(&error))?;
    Ok(to_user_summary(&record))
}

/// `users_update` command: role/active-flag patch, last-admin protected.
#[tauri::command(rename_all = "snake_case")]
pub fn users_update(
    state: tauri::State<'_, AppState>,
    request: UpdateUserRequestDto,
) -> Result<UserSummaryDto, String> {
    let session = require_session(&state, &request.session_token)?;
    let user_id = parse_wire_id(&request.user_id, "user_id")
        .map(baraka_domain::user::UserId::from)
        .map_err(|error| error_code(&error))?;
    let role = request
        .role
        .as_deref()
        .map(|code| {
            Role::from_code(code.trim())
                .map_err(|error| error_code(&ApplicationError::Validation(error)))
        })
        .transpose()?;
    let record = UpdateUserUseCase::new(state.users.as_ref(), state.clock.as_ref())
        .execute(&session, user_id, role, request.is_active)
        .map_err(|error| error_code(&error))?;
    Ok(to_user_summary(&record))
}

/// `users_reset_credentials` command: password/PIN replacement (UserManage-gated).
#[tauri::command(rename_all = "snake_case")]
pub fn users_reset_credentials(
    state: tauri::State<'_, AppState>,
    request: ResetUserCredentialsRequestDto,
) -> Result<(), String> {
    let session = require_session(&state, &request.session_token)?;
    let user_id = parse_wire_id(&request.user_id, "user_id")
        .map(baraka_domain::user::UserId::from)
        .map_err(|error| error_code(&error))?;
    ResetUserCredentialsUseCase::new(
        state.hasher.as_ref(),
        state.users.as_ref(),
        state.clock.as_ref(),
    )
    .execute(
        &session,
        user_id,
        request.password.as_deref(),
        request.pin.as_deref(),
    )
    .map_err(|error| error_code(&error))
}

/// Map an application user record onto its wire summary (hashes never cross).
fn to_user_summary(record: &baraka_application::ports::UserRecord) -> UserSummaryDto {
    UserSummaryDto {
        id: record.id.to_string(),
        username: record.username.clone(),
        role: record.role.code().to_string(),
        is_active: record.is_active,
        has_pin: record.has_pin,
        created_at: record.created_at.clone(),
        updated_at: record.updated_at.clone(),
    }
}
