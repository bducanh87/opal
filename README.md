# Opal

**Opal** là bản viết lại bằng Rust của **Lumen 0.1.0** (MIT): các widget kính mờ nằm ngay trên hình nền Windows. Bản này thay Electron + Python + .NET bằng **một file `opal.exe` (~10 MB)**, không cần cài đặt gì thêm.

## Chạy

```bash
cargo build --release
```

Sau khi build, chạy `target\release\opal.exe`. Khi chạy, app tạo thư mục `config\` ngay cạnh exe (portable, giống Lumen gốc). Nếu thư mục đó không ghi được, app dùng `%APPDATA%\Opal`.

Tham số dòng lệnh:
- `--settings`: mở cửa sổ cài đặt khi khởi động.
- `--widgets`: mở bảng Widgets khi khởi động.

- `--show weather,system,…`: bật các widget đó khi khởi động.

Biến môi trường `OPAL_CONFIG_DIR` dùng để chỉ định thư mục config khác.

## Tính năng

| Widget | Có gì |
|---|---|
| **Launcher (App Dock)** | Ô tìm kiếm dạng viên thuốc: tìm mọi app trong Start Menu và app Store (có bỏ dấu tiếng Việt), Enter để mở, ghim app. Lưới icon lấy từ shell của Windows, kéo để đổi thứ tự, thả file/thư mục vào để thêm, ô **Add** nét đứt. Menu chuột phải: Mở, Đổi tên, Hiện trong Explorer, Di chuyển, Xoá. Các nút trên ô tìm kiếm: bảng Widgets, bật/tắt Nhạc, Ghi chú, Lịch, Cài đặt. |
| **Bảng Widgets** | Danh sách widget kèm trạng thái (ON DESKTOP / HIDDEN), công tắc bật/tắt và nút cài đặt riêng cho từng widget. |
| **Clock** | 9 mặt đồng hồ như Lumen gốc: Opal, Afterglow, Paper, Nightfall, Colour block, Split flap, Daybook, Timeline, Terminal. Hỗ trợ 12/24 giờ, hiện giây, múi giờ IANA. |
| **Notes** | Pinboard / Notebook / Index. 7 màu, ghim lên đầu, xoá có Hoàn tác, tự lưu khi gõ. |
| **Calendar** | Opal / Almanac / Blocks / Pop / Prism / Grove. Đồng bộ lịch qua link **ICS** (Google, Outlook), hỗ trợ sự kiện lặp (RRULE DAILY/WEEKLY/MONTHLY/YEARLY, BYDAY, COUNT, UNTIL, EXDATE). |
| **Music** | Compact / Sleeve / Turntable. Điều khiển **mọi** trình phát qua Windows Media Session (Spotify, YouTube Music trên trình duyệt, Media Player, VLC…): phát/dừng, bài trước/sau, tua, ảnh bìa. |
| **Weather** *(mới)* | Nhiệt độ, trạng thái trời, cao/thấp, độ ẩm, gió, dự báo theo giờ (hoặc theo ngày khi widget cao). Dữ liệu Open-Meteo, không cần API key. Thành phố đặt trong cài đặt riêng; để trống thì tự định vị qua địa chỉ IP. |
| **System** *(mới)* | CPU kèm biểu đồ 60 giây, RAM, ổ hệ thống, tốc độ mạng lên/xuống, thời gian máy đã chạy. |
| **Focus timer** *(mới)* | Pomodoro: Focus / Short / Long, vòng tiến trình, Start/Pause, Reset, Skip; nghỉ dài sau mỗi 4 phiên; báo âm thanh và thông báo khi hết giờ. Thời lượng chỉnh trong cài đặt riêng. |
| **Photos** *(mới)* | Trình chiếu ảnh trong một thư mục (mặc định Pictures), chuyển mờ dần, rê chuột để xem tên ảnh và chuyển ảnh. |
| **Âm lịch** *(mới)* | Ngày dương và ngày âm, can chi ngày/tháng/năm, tiết khí, giờ hoàng đạo (thuật toán Hồ Ngọc Đức, múi giờ +7). |
| **To-do** *(mới)* | Danh sách việc: thêm, đánh dấu xong, xoá, dọn việc đã xong. Tự lưu. |
