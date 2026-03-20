# EspFlashTool - Thông tin dự án

## Tổng quan dự án (Project Overview)
**EspFlashTool** là một ứng dụng desktop được xây dựng bằng framework **Tauri**. Ứng dụng này sử dụng kiến trúc kết hợp giữa frontend và backend để mang lại hiệu suất cao và giao diện người dùng hiện đại.

*   **Frontend:** Sử dụng **Vue 3** kết hợp với **TypeScript**, được đóng gói và phát triển thông qua **Vite**. Kiến trúc frontend bao gồm các hàm API wrapper nằm trong thư mục `src/api` để tương tác trực tiếp với backend.
*   **Backend:** Sử dụng ngôn ngữ **Rust**, cung cấp độ an toàn và tốc độ xử lý vượt trội.
*   **Kiến trúc:** Dự án được thiết kế theo kiến trúc **Sidecar & Commands**:
    *   **Commands:** Các hàm xử lý Rust được module hóa trong `src-tauri/src/commands/` và được gọi từ frontend qua Tauri API.
    *   **Sidecars:** Được thiết kế để chạy các tệp thực thi bên ngoài (ví dụ: `esptool`), được định nghĩa trong `tauri.conf.json` và giao tiếp qua plugin `@tauri-apps/plugin-shell`. Frontend gọi các sidecar thông qua `src/api/sidecars.ts`.

## Cấu trúc thư mục chính (Directory Structure)
*   `src/`: Chứa mã nguồn frontend (Vue components, TypeScript, assets).
    *   `src/api/`: Chứa các hàm giao tiếp với Tauri backend (`commands.ts`, `sidecars.ts`).
*   `src-tauri/`: Chứa mã nguồn backend (Rust) và cấu hình Tauri.
    *   `src-tauri/src/commands/`: Nơi chứa các lệnh Rust (Tauri Commands).
    *   `src-tauri/binaries/`: Nơi chứa các tệp thực thi bên ngoài cho Sidecar.
    *   `src-tauri/capabilities/`: Quản lý phân quyền (permissions) của ứng dụng Tauri (vd: truy cập shell, file system).
*   `tauri.conf.json`: Tệp cấu hình chính của Tauri.

## Xây dựng và Chạy ứng dụng (Building and Running)

Dự án sử dụng `npm` làm trình quản lý gói.

*   **Cài đặt các gói phụ thuộc (Dependencies):**
    ```bash
    npm install
    ```
*   **Chạy môi trường phát triển (Development):**
    Chạy frontend (Vite) và backend (Tauri) đồng thời, có hỗ trợ hot-reload:
    ```bash
    npm run tauri dev
    ```
    Hoặc:
    ```bash
    npx tauri dev
    ```
*   **Biên dịch ứng dụng (Build for Production):**
    Tạo bản phát hành (installer/bundle) cho hệ điều hành hiện tại:
    ```bash
    npm run tauri build
    ```

## Quy ước phát triển (Development Conventions)
1.  **Giao tiếp Frontend - Backend:** Tất cả các thao tác gọi lệnh Rust (Commands) hoặc chạy tiến trình ngoại vi (Sidecar) từ Vue components **phải** được thực hiện thông qua các hàm đã được khai báo sẵn trong thư mục `src/api/` (như `commands.ts` và `sidecars.ts`), thay vì gọi trực tiếp API Tauri trong components.
2.  **Quản lý quyền (Capabilities):** Bất cứ khi nào thêm một tính năng mới yêu cầu truy cập hệ thống (như đọc/ghi file, thực thi shell, mạng), cần phải cập nhật tệp cấu hình trong `src-tauri/capabilities/` tương ứng.
3.  **Môi trường:** Dự án sử dụng TypeScript nghiêm ngặt (Strict mode). Hãy đảm bảo định nghĩa kiểu dữ liệu (Types/Interfaces) rõ ràng cho dữ liệu trả về từ Rust Commands.
4.  **Ngôn ngữ:** Luôn phản hồi và giải thích các thay đổi bằng **Tiếng Việt**.
