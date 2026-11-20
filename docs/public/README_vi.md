<h1 align="center">
  <img src="assets/logo.png" alt="Verso Logo" width="128" height="128"><br>
  Verso Translate
</h1>

<p align="center">
  <img src="https://camo.githubusercontent.com/782429a6a8e7c3c86ca73fe31edaa6a48e78379e77e8c6c2a9637f67f475092c/68747470733a2f2f696d672e736869656c64732e696f2f6769746875622f646f776e6c6f6164732f6c626e676f632f766572736f2d7472616e736c6174652f746f74616c3f6c6162656c3d446f776e6c6f616473" alt="Downloads" />
  <img src="https://camo.githubusercontent.com/f0cb8ba13592d31161791daf5df230b9e7c063b00e304ac5a2f0661e7e2c8a57/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f4c6963656e73652d4d49542d626c75652e737667" alt="License: MIT" />
  <img src="https://camo.githubusercontent.com/2f3c09ff456bca9042c7e963a35e2a779cfd8d1a0558f31b1e8073c28223f1cc/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f506c6174666f726d2d6d61634f532d3030303030303f6c6f676f3d6170706c65266c6f676f436f6c6f723d7768697465" alt="Platform: macOS" />
  <img src="https://camo.githubusercontent.com/b1601f2fbc84f5ddd0396e874463e38a59c66eafa42a2ca1a155ed86883c9c16/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f506c6174666f726d2d57696e646f77732d3030373844343f6c6f676f3d77696e646f7773266c6f676f436f6c6f723d7768697465" alt="Platform: Windows" />
</p>

<p align="center">
  <strong>Ứng dụng dịch nhỏ gọn cho văn bản đã chọn hoặc tự nhập, trên macOS và Windows.</strong><br>
  Chọn, nhấn phím tắt, đọc. Không tài khoản. Không telemetry.
</p>

---

## 📥 Tải về & Cài đặt

### 🍺 Homebrew (macOS)

```bash
brew install --cask lbngoc/verso/verso
```

> **Cập nhật:** Verso tự cập nhật sau khi bạn đồng ý kiểm tra hằng ngày nên không cần `brew upgrade`
> (cask đặt `auto_updates`).

### 📦 Tải thủ công

|  Hệ điều hành  | Trạng thái |                                         Tải xuống                                          | Hướng dẫn                         |
| :------------: | :--------: | :----------------------------------------------------------------------------------------: | :-------------------------------- |
| **macOS** (Apple Silicon) | ✅ Sẵn sàng | [📥 Verso_aarch64.dmg](https://github.com/lbngoc/verso-translate/releases/latest/download/Verso_aarch64.dmg) | [Xem hướng dẫn](INSTALL.md#macos) |
| **macOS** (Intel) | ✅ Sẵn sàng | [📥 Verso_x64.dmg](https://github.com/lbngoc/verso-translate/releases/latest/download/Verso_x64.dmg) | [Xem hướng dẫn](INSTALL.md#macos) |
| **Windows** (64-bit) | ✅ Sẵn sàng | [📥 Verso_x64-setup.exe](https://github.com/lbngoc/verso-translate/releases/latest/download/Verso_x64-setup.exe) | [Xem hướng dẫn](INSTALL.md#windows) |

Mỗi bản phát hành kèm [`SHA256SUMS.txt`](https://github.com/lbngoc/verso-translate/releases/latest/download/SHA256SUMS.txt); lệnh kiểm tra nằm trong
[INSTALL.md](INSTALL.md).

<details>
<summary><strong>⚠️ macOS: app không mở được?</strong></summary>

Bản cài được ký ad-hoc nhưng chưa notarize, nên macOS chặn một lần với thông báo "Apple could not verify
'Verso' is free of malware".

1. Bấm **Done** (không chọn **Move to Trash**).
2. Mở **System Settings › Privacy & Security**, cuộn xuống **Security**, bấm **Open Anyway** cạnh Verso rồi
   nhập mật khẩu. Nút chỉ hiện khoảng một giờ sau lần mở bị chặn.
3. Trên macOS 13 và 14 có thể chuột phải `Verso` trong `Applications` rồi chọn **Open**.
4. Bật Verso trong **Privacy & Security › Accessibility** để app đọc được văn bản đã chọn.

Cách dùng Terminal nằm trong [INSTALL.md](INSTALL.md).

</details>

<details>
<summary><strong>⚠️ Windows: SmartScreen chặn bộ cài?</strong></summary>

Bản Windows chưa được ký số. Ở màn hình "Windows protected your PC", chọn **More info** rồi **Run anyway**.
Bộ cài không cần quyền quản trị.

</details>

## ✨ Tính năng

### 🔥 Nổi bật

- ⌨️ **Chọn, nhấn, đọc** — `Alt+Shift+Z` hiện tooltip nhỏ cạnh vùng chọn rồi lớn dần thành bản dịch
- 🔊 **Đọc** — `Alt+Shift+S` đọc văn bản đã chọn mà không mở cửa sổ
- 🪟 **Bảng dịch nhanh** — `Alt+Shift+T` (hoặc bấm biểu tượng khay) mở bảng cho văn bản bạn tự nhập; `Tab` đổi ngôn ngữ
- 🔁 **Hai engine, tự chuyển khi lỗi** — Google Translate và Microsoft Translator (Bing); không cần tài khoản hay API key
- 🎯 **Popup bấm phím là dùng** — nhận phím ngay khi có kết quả: `c` sao chép, `s` đọc, `-` `=` `0` cỡ chữ
- 🔄 **Tự cập nhật** — bản tải về được ký và xác minh, sau khi bạn đồng ý kiểm tra hằng ngày

### 📋 Đầy đủ

- 🌐 **Tự nhận diện ngôn ngữ nguồn**, hàng chục ngôn ngữ đích
- 🗣️ **Giao diện tiếng Anh hoặc tiếng Việt**
- 🎨 **Chủ đề sáng/tối**, màu nhấn teal của Verso hoặc màu hệ thống
- ⚙️ **Ghi phím tắt** — đổi mọi phím trong **Cài đặt › Phím tắt**; bản cập nhật không đổi phím bạn đã lưu
- 📋 **Điền sẵn từ clipboard** — bật tuỳ chọn để dán clipboard vào bảng dịch nhanh
- 🐞 **Báo lỗi** từ khay hoặc Cài đặt, phần cuối log được điền sẵn vào form issue

### 🛡️ Quyền riêng tư

- 🚫 **Không telemetry** — không thu thập gì về bạn
- 🚫 **Không tài khoản** — không đăng nhập, không API key
- 🔎 **Rõ văn bản đi đâu** — chỉ engine bạn dùng nhận văn bản; xem [PRIVACY.md](PRIVACY.md)

---

## 📚 Tài liệu

Các tài liệu chi tiết hiện bằng tiếng Anh:

- [INSTALL.md](INSTALL.md) — cài trên macOS và Windows, checksum, quyền, cập nhật
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — engine bị chặn, phím tắt, log, cập nhật, báo lỗi
- [PRIVACY.md](PRIVACY.md) — engine nào nhận văn bản của bạn
- [TERMS.md](TERMS.md) — điều khoản cho bản binary
- [CHANGELOG.md](CHANGELOG.md) — thay đổi từng bản
- [THIRD_PARTY.md](THIRD_PARTY.md) — thông báo bên thứ ba
- [SECURITY.md](SECURITY.md) — báo lỗi bảo mật riêng tư

---

## 📄 Giấy phép

Mã nguồn mở tại [github.com/lbngoc/verso](https://github.com/lbngoc/verso) theo
[giấy phép MIT](https://github.com/lbngoc/verso/blob/main/LICENSE); issue và pull request được chào đón ở đó.
Kho này chứa tài liệu công khai và bản tải về. Điều khoản áp dụng cho bản binary nằm ở [TERMS.md](TERMS.md).

Copyright © 2026 Ngoc Luong.

---

[English](README.md)
