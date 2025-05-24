# iLet Nightscout Synchronizer (iLetSync-Rust)

**Version: 0.2.0**

This command-line tool fetches clinical data from iLet Bionic Pancreas servers and uploads relevant insulin treatment data (basal and bolus) to a specified Nightscout site.

It is designed for users of the iLet device who wish to integrate their insulin data with Nightscout for comprehensive diabetes management and visualization.

## Features

-   Fetches iLet clinical data (CGM values, insulin delivery, etc.) for a specified date range. Important limitation: There is an as-of-yet undefined 'maximum' date range the iLet report will return, avoid date ranges larger than 3 months for now.
-   Converts iLet basal and bolus insulin data into Nightscout treatment objects.
-   Uploads these treatments to your Nightscout site.
-   Configuration via a `config.json` file and/or command-line arguments.
-   Date ranges are mandatory and can be specified via CLI or `config.json`.

## Configuration

Configuration is primarily handled through a `config.json` file located in the same directory as the executable. Command-line arguments can be used to override date settings.

### `config.json` File Structure

The `config.json` file should follow this structure:

```json
{
  "iLet": {
    "username": "your_ilet_username",
    "password": "your_ilet_password",
    "serial_number": "your_device_serial_number",
    "startDate": "YYYY-MM-DDTHH:MM:SSZ", // Optional: e.g., "2023-01-01T00:00:00Z"
    "endDate": "YYYY-MM-DDTHH:MM:SSZ"    // Optional: e.g., "2023-01-02T23:59:59-05:00"
  },
  "Nightscout": {
    "website": "https://your-nightscout-instance.herokuapp.com/", // Include trailing slash
    "permission_role": "devicestatus-json" 
    // Or the specific role you've set up in Nightscout Access Control (e.g., "api-secret", "admin", "devicestatus-json")
    // If using an API secret (token) directly, ensure your Nightscout setup expects it for the /api/v3/treatments endpoint.
    // This tool uses role-based token generation via /api/v2/authorization/request.
  }
}
```

**Important Notes on `config.json`:**

-   **`iLet.username` / `iLet.password`**: Your login credentials for the iLet system.
-   **`iLet.serial_number`**: The serial number of your iLet device.
-   **`iLet.startDate` / `iLet.endDate`**: These are used if command-line date arguments are not supplied.
-   **`Nightscout.permission_role`**: The access role used to generate an API token. Common roles are `devicestatus-json` (read-only for some data, might need adjustment for writing treatments) or a custom role with `api:*:*` or `api:v3:treatments:*` permissions.

### Command-Line Arguments

Command-line arguments can be used to specify the date range for fetching data. If provided, these will override any `startDate` and `endDate` values in `config.json`.

-   `--start-date <YYYY-MM-DDTHH:MM:SSZ>`: Sets the start date/time for data fetching.
-   `--end-date <YYYY-MM-DDTHH:MM:SSZ>`: Sets the end date/time for data fetching.

**Date ranges are mandatory.** You must provide them either via command-line arguments or in the `config.json` file. If using command-line arguments, both `--start-date` and `--end-date` must be specified.

## Usage

1.  **Prepare `config.json`**: Create and populate `config.json` with your iLet and Nightscout details.
2.  **Run the application**:
    -   If dates are in `config.json`:
        ```bash
        ./iLetSync-Rust 
        # (Or iLetSync-Rust.exe on Windows)
        ```
    -   To specify dates via command line (overrides config dates):
        ```bash
        ./iLetSync-Rust --start-date "2023-05-01T00:00:00Z" --end-date "2023-05-01T23:59:59Z"
        ```
    -   Using a specific timezone (e.g., EST which is UTC-5):
        ```bash
        ./iLetSync-Rust --start-date "2023-05-01T00:00:00-05:00" --end-date "2023-05-01T23:59:59-05:00"
        ```

The application will print informational messages about its progress, including dates being used, authentication status, number of records fetched, and status of uploads to Nightscout. Errors will be printed to the console.

## Building from Source

To build from source:

1.  Ensure you have Rust installed (see [rustup.rs](https://rustup.rs/)).
2.  Clone the repository (if applicable).
3.  Navigate to the project directory.
4.  Build the release executable:
    ```bash
    cargo build --release
    ```
5.  The executable will be located at `target/release/iLetSync-Rust` (or `target\release\iLetSync-Rust.exe` on Windows). You can copy this to a preferred location.

## Known Limitations & Important Considerations

-   **iLet API**: The interaction with iLet servers is based on reverse-engineering of their private API. This API could change at any time without notice, potentially breaking this tool. Use at your own risk.
-   **Nightscout API - Treatment Uploads**: This tool uploads treatments (basal, bolus, and food currently) as individual API requests to Nightscout. This is due to observed difficulties with bulk treatment upload endpoints in the Nightscout API (v3) at the time of development. While less efficient, it's more reliable.
-   **Error Handling**: Critical errors will cause the application to exit. Non-critical errors during the upload of individual records will be printed, but the application will attempt to continue with other records.
-   **Data Interpretation**: The conversion from iLet data to Nightscout treatments involves some interpretation/creative liberties(e.g., how iLet's food 'categories' map to Nightscout's food event types). Review the data in Nightscout to ensure it aligns with your expectations.
-   **Security**: Your iLet credentials are stored in `config.json`. Ensure this file is kept secure. Avoid committing it to public repositories.

## Contributing

Contributions or suggestions are welcome! Please feel free to open an issue or submit a pull request if you have ideas for improvements or bug fixes.

## License

This project is licensed under the terms of the AGPLv3 license. See `LICENSE` file for details.
