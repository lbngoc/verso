# Verso Translate

> **Bản xem trước** — `0.1.0` là bản preview công khai đầu tiên cho macOS và Windows. Ứng dụng tự cập nhật;
> Linux, bản ký số và winget sẽ đến trong các bản cập nhật thường kỳ. Homebrew đã có sẵn.

Verso là ứng dụng dịch nhỏ sống trên thanh menu (macOS) hoặc khay hệ thống (Windows). Chọn văn bản ở bất kỳ
ứng dụng nào và nhấn `Alt+Shift+T`: Verso dịch và hiện kết quả trong popup có thể sao chép hoặc đọc. Văn bản
bạn tự gõ dùng bảng nhập nhanh.

- **Engine**: Google Translate và Microsoft Translator (Bing), không cần tài khoản hay API key, tự động
  chuyển engine khi một bên lỗi.
- **Tự nhận diện ngôn ngữ nguồn**, hàng chục ngôn ngữ đích, giao diện tiếng Anh hoặc tiếng Việt.
- **Chủ đề sáng/tối**; biểu tượng khay theo hệ thống.
- **Tự cập nhật** sau khi bạn đồng ý kiểm tra hằng ngày; bản tải về được ký và xác minh.
- **Báo lỗi** từ khay hoặc Cài đặt, phần cuối log được điền sẵn vào form issue.
- **Không telemetry**, không tài khoản. Xem [PRIVACY.md](PRIVACY.md).

## Cài đặt

### Homebrew (macOS)

```sh
brew tap lbngoc/verso
brew install --cask verso
```

Cask tự xoá cờ quarantine sau khi cài nên bản preview mở được ngay, không hiện cảnh báo Gatekeeper. Không cần
`brew upgrade` (cask đặt `auto_updates`): Verso tự cập nhật.

### Tải trực tiếp

Bản cài mới nhất ở trang [Releases](../../releases/latest):

| Hệ điều hành | Tệp |
|---|---|
| macOS 13 trở lên, Apple Silicon | `Verso_<version>_aarch64.dmg` |
| macOS 13 trở lên, Intel | `Verso_<version>_x64.dmg` |
| Windows 10/11, 64-bit | `Verso_<version>_x64-setup.exe` |

Mỗi bản phát hành kèm `SHA256SUMS.txt` chứa SHA-256 của từng tệp.

**Lần mở đầu trên macOS.** Bản preview được ký ad-hoc nhưng chưa notarize, nên macOS chặn một lần với thông
báo "Apple could not verify 'Verso' is free of malware". Bấm **Done**, mở **System Settings › Privacy &
Security**, cuộn xuống **Security**, bấm **Open Anyway** cạnh Verso rồi nhập mật khẩu. Nút chỉ hiện khoảng
một giờ sau lần mở bị chặn. Trên macOS 13 và 14 có thể chuột phải `Verso` trong `Applications` rồi chọn
**Open**. Nếu không thấy nút, xem cách dùng Terminal trong [INSTALL.md](INSTALL.md).

**Windows.** SmartScreen hiện "Windows protected your PC" một lần: chọn **More info** → **Run anyway**.
Winget sẽ được nộp sau bản preview; trong lúc đó dùng bộ cài x64 và để ứng dụng tự cập nhật.

Dù cài bằng kênh nào, Verso vẫn tự cập nhật.

## Tài liệu

Các tài liệu chi tiết hiện bằng tiếng Anh:

- [INSTALL.md](INSTALL.md) — cài trên macOS và Windows, checksum, quyền, cập nhật
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — engine bị chặn, phím tắt, log, cập nhật, báo lỗi
- [PRIVACY.md](PRIVACY.md) — engine nào nhận văn bản của bạn
- [TERMS.md](TERMS.md) — điều khoản cho bản binary
- [CHANGELOG.md](CHANGELOG.md) — thay đổi từng bản
- [THIRD_PARTY.md](THIRD_PARTY.md) — thông báo bên thứ ba
- [SECURITY.md](SECURITY.md) — báo lỗi bảo mật riêng tư

## Ảnh chụp màn hình

Ảnh popup, bảng nhập nhanh và cửa sổ Cài đặt sẽ được thêm sau bản công khai đầu tiên.

## Trạng thái dự án

Kho này chứa tài liệu công khai và bản tải về. Mã nguồn chưa mở nên pull request bị tắt; issue được bật để báo
lỗi và đặt câu hỏi. Điều khoản áp dụng cho bản binary nằm ở [TERMS.md](TERMS.md).

---

[English](README.md)
