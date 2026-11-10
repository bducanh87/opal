# Opal (bản egui)

**Opal** là bộ widget kính mờ nằm ngay trên hình nền Windows: thanh ứng dụng, đồng hồ, ghi chú, lịch, nhạc, thời tiết, âm lịch… Opal là bản viết lại bằng Rust của **Lumen 0.1.0** (giấy phép MIT), thay Electron + Python + .NET bằng **một file `opal.exe` khoảng 10 MB**, không cần cài đặt gì thêm.

Bản này vẽ giao diện bằng [egui](https://github.com/emilk/egui). Còn một bản dùng Tauri (giao diện HTML/CSS) ở repo [`opal-tauri`](https://github.com/bducanh87/opal-tauri). Bản Tauri là bản đang được phát triển thêm: theme Opal, widget Đếm ngược, YouTube Music có danh sách phát, hiệu ứng theo nhạc. Hai bản không chạy cùng lúc: bản mở sau sẽ báo và tự thoát.

## Chạy

Cần có [Rust](https://rustup.rs) (bản stable) và Windows 10/11.

```bash
cargo build --release
```

Sau khi build, chạy `target\release\opal.exe`. Icon Opal xuất hiện ở khay hệ thống, các widget hiện trên hình nền.

Tham số dòng lệnh:
- `--settings`: mở cửa sổ cài đặt khi khởi động.
- `--widgets`: mở bảng Widgets khi khởi động.
- `--widget-settings <id>`: mở cài đặt của một widget (ví dụ `clock`, `music`).
- `--show weather,system,…`: bật các widget đó khi khởi động.

## Hướng dẫn sử dụng

| Muốn làm gì | Cách làm |
|---|---|
| Xem các widget | Ra màn hình desktop (`Win + D`). Widget luôn nằm sát hình nền, dưới các cửa sổ khác. |
| Bật / tắt widget | Mở bảng **Widgets** (icon ở khay hệ thống, hoặc nút trên ô tìm kiếm của App Dock), gạt công tắc từng widget. |
| Di chuyển widget | Nhấn giữ vào vùng trống của widget rồi kéo. |
| Đổi kích thước | Kéo góc dưới bên phải của widget. |
| Cài đặt riêng một widget | Rê chuột lên widget, bấm bánh răng. Mỗi widget có kiểu thiết kế (Design), theme và thông số kính riêng. |
| Thêm ứng dụng vào App Dock | Gõ tên trong ô tìm kiếm rồi ghim, thả file/thư mục vào, hoặc bấm ô **Add**. |
| Lấy dữ liệu từ Lumen gốc | Cài đặt → **Import from Lumen…**, chọn thư mục Lumen (hoặc thư mục `config` của nó). |
| Thoát Opal | Chuột phải icon ở khay hệ thống → **Quit Opal**. |

## Dữ liệu lưu ở đâu

Opal tạo thư mục `config\` ngay cạnh file exe (kiểu portable, giống Lumen gốc). Nếu thư mục đó không ghi được, app dùng `%APPDATA%\Opal`. Biến môi trường `OPAL_CONFIG_DIR` dùng để chỉ định thư mục khác.

| File | Nội dung |
|---|---|
| `state.json` | Ứng dụng ghim trong App Dock và toàn bộ cài đặt (cùng định dạng với Lumen gốc). |
| `notes.json` | Ghi chú. |
| `calendar.json` | Link ICS và cài đặt lịch. |
| `opal-extras.json` | Việc cần làm, cài đặt thời tiết, ảnh, Pomodoro… |
| `opal-egui.json` | Vị trí, kích thước, trạng thái bật tắt của từng widget. |
| `opal-egui.log` | Nhật ký. |

## Tính năng

| Widget | Có gì |
|---|---|
| **Launcher (App Dock)** | Ô tìm kiếm dạng viên thuốc: tìm mọi app trong Start Menu và app Store (có bỏ dấu tiếng Việt), Enter để mở, ghim app. Lưới icon lấy từ shell của Windows, kéo để đổi thứ tự, thả file/thư mục vào để thêm, ô **Add** nét đứt. Menu chuột phải: Mở, Đổi tên, Hiện trong Explorer, Di chuyển, Xoá. Các nút trên ô tìm kiếm: bảng Widgets, bật/tắt Nhạc, Ghi chú, Lịch, Cài đặt. |
| **Bảng Widgets** | Danh sách widget kèm trạng thái (ON DESKTOP / HIDDEN), công tắc bật/tắt và nút cài đặt riêng cho từng widget. |
| **Clock** | 9 mặt đồng hồ như Lumen gốc: Opal, Afterglow, Paper, Nightfall, Colour block, Split flap, Daybook, Timeline, Terminal. Hỗ trợ 12/24 giờ, hiện giây, múi giờ IANA. |
| **Notes** | Pinboard / Notebook / Index. 7 màu, ghim lên đầu, xoá có Hoàn tác, tự lưu khi gõ. |
| **Calendar** | Opal / Almanac / Blocks / Pop / Prism / Grove. Đồng bộ lịch qua link **ICS** (Google, Outlook), hỗ trợ sự kiện lặp (RRULE DAILY/WEEKLY/MONTHLY/YEARLY, BYDAY, COUNT, UNTIL, EXDATE). |
| **Music** | Coverflow / Compact / Sleeve / Turntable. Điều khiển **mọi** trình phát qua Windows Media Session (Spotify, YouTube Music trên trình duyệt, Media Player, VLC…): phát/dừng, bài trước/sau, tua, ảnh bìa. |
| **Weather** *(mới)* | Nhiệt độ, trạng thái trời, cao/thấp, độ ẩm, gió, dự báo theo giờ (hoặc theo ngày khi widget cao). Dữ liệu Open-Meteo, không cần API key. Thành phố đặt trong cài đặt riêng; để trống thì tự định vị qua địa chỉ IP. |
| **System** *(mới)* | CPU kèm biểu đồ 60 giây, RAM, ổ hệ thống, tốc độ mạng lên/xuống, thời gian máy đã chạy. |
| **Focus timer** *(mới)* | Pomodoro: Focus / Short / Long, vòng tiến trình, Start/Pause, Reset, Skip; nghỉ dài sau mỗi 4 phiên; báo âm thanh và thông báo khi hết giờ. Thời lượng chỉnh trong cài đặt riêng. |
| **Photos** *(mới)* | Trình chiếu ảnh trong một thư mục (mặc định Pictures), chuyển mờ dần, rê chuột để xem tên ảnh và chuyển ảnh. |
| **Âm lịch** *(mới)* | Ngày dương và ngày âm, can chi ngày/tháng/năm, tiết khí, giờ hoàng đạo (thuật toán Hồ Ngọc Đức, múi giờ +7). |
| **To-do** *(mới)* | Danh sách việc: thêm, đánh dấu xong, xoá, dọn việc đã xong. Tự lưu. |

## Chạy test

```bash
cargo test --release
```

## Nguồn gốc và giấy phép

Opal được viết lại từ **Lumen 0.1.0**, một ứng dụng phát hành theo giấy phép MIT. Thuật toán âm lịch theo công bố của Hồ Ngọc Đức.

Mã nguồn Opal phát hành theo giấy phép MIT, xem file [LICENSE](LICENSE). Phần kế thừa từ Lumen được ghi trong file [NOTICE](NOTICE).
