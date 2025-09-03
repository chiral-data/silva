# Feature: Health Check

## 1. Summary

This feature introduces a health check mechanism to validate Silva's configuration, ensuring successful operation. It aims to simplify the current, often complex, configuration method for end-users.

## 2. User Goal

For end users to easily verify their Silva setup.

## 3. UI Behavior

- **Location:** A new "Health Check" tab.
- **Appearance:**
  - A list of configuration items to be checked.
  - Each item will display a green checkmark upon success, or a red cross upon failure.
- **Interaction:**
  - Users can manually rerun the health check, and the UI will update with the latest results.

## 4. Tasks

### Remove the old configuration method

### Build the UI

- [x] 2025-09-03 "HealthCheck" component under the "Welcome" page completed

### Establish configuration checking rules

- [ ] Verify if both `SILVA_CHIRAL_USERNAME` and `SILVA_CHIRAL_API_TOKEN` environment variables are set. If they are, the chiral service will be utilized.
- [ ] If the above environment variables are not set, confirm that _Docker_ is installed locally, as the local computer will be used for processing.

## 5. Acceptance Criteria

## 6. Scope
