/*!
 * Trumbowyg File Manager — RustAdmin port of NodeAdmin's custom plugin (1:1 behaviour).
 * The "filemanager" toolbar button opens a modal that: lists images from storage
 * (GET /admin/v1/media/list), uploads new ones (POST .../upload), deletes
 * (POST .../delete), and inserts an <img> into the editor when a thumbnail is clicked.
 *
 * CSRF: the token is read from <meta name="csrf-token"> and sent via the
 * x-csrf-token header (validated server-side by the `CsrfProtected` request guard).
 *
 * Requires jQuery + Trumbowyg (loaded in head.tera). The modal uses its own markup +
 * styles (no Bootstrap JS) so it carries no extra dependency.
 */
(function ($) {
    'use strict'
    if (!$ || !$.trumbowyg) return

    var BASE = '/admin/v1/media'

    function csrfToken() {
        var meta = document.querySelector('meta[name="csrf-token"]')
        return meta ? meta.getAttribute('content') : ''
    }

    // --- Modal (injected once) ---
    function ensureModal() {
        if (document.getElementById('tbFmModal')) return
        var html =
            '<div id="tbFmModal" class="tb-fm-overlay" style="display:none">' +
            '  <div class="tb-fm-dialog">' +
            '    <div class="tb-fm-header">' +
            '      <strong><i class="fas fa-images"></i> File Manager</strong>' +
            '      <button type="button" class="tb-fm-close" aria-label="Tutup">&times;</button>' +
            '    </div>' +
            '    <div class="tb-fm-body">' +
            '      <form id="tbFmUploadForm" class="tb-fm-uploadbar">' +
            '        <input type="file" name="file" accept="image/*" required>' +
            '        <button type="submit" class="tb-fm-btn-primary">Upload</button>' +
            '      </form>' +
            '      <p class="tb-fm-hint">Klik gambar untuk menyisipkan ke editor.</p>' +
            '      <div id="tbFmList" class="tb-fm-grid"></div>' +
            '    </div>' +
            '  </div>' +
            '</div>'
        var wrap = document.createElement('div')
        wrap.innerHTML = html
        document.body.appendChild(wrap.firstChild)

        var css =
            '.tb-fm-overlay{position:fixed;inset:0;background:rgba(0,0,0,.5);z-index:11000;display:flex;align-items:center;justify-content:center}' +
            '.tb-fm-dialog{background:#fff;border-radius:8px;width:min(720px,92vw);max-height:88vh;display:flex;flex-direction:column;box-shadow:0 10px 40px rgba(0,0,0,.3)}' +
            '.tb-fm-header{display:flex;align-items:center;justify-content:space-between;padding:14px 18px;border-bottom:1px solid #eee}' +
            '.tb-fm-close{border:0;background:none;font-size:24px;line-height:1;cursor:pointer;color:#888}' +
            '.tb-fm-body{padding:16px 18px;overflow:auto}' +
            '.tb-fm-uploadbar{display:flex;gap:8px;margin-bottom:10px}' +
            '.tb-fm-uploadbar input[type=file]{flex:1;border:1px solid #ddd;border-radius:6px;padding:6px}' +
            '.tb-fm-btn-primary{background:#2563eb;color:#fff;border:0;border-radius:6px;padding:6px 16px;cursor:pointer;white-space:nowrap}' +
            '.tb-fm-btn-primary:disabled{opacity:.6;cursor:default}' +
            '.tb-fm-hint{color:#888;font-size:12px;margin:6px 0 12px}' +
            '.tb-fm-grid{display:flex;flex-wrap:wrap;gap:12px}' +
            '.tb-fm-item{display:flex;flex-direction:column;align-items:center;width:120px}' +
            '.tb-fm-thumb{width:120px;height:90px;object-fit:cover;border:1px solid #ddd;border-radius:6px;cursor:pointer}' +
            '.tb-fm-thumb:hover{outline:2px solid #2563eb}' +
            '.tb-fm-name{font-size:11px;color:#666;max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin-top:4px}' +
            '.tb-fm-del{margin-top:4px;border:1px solid #dc2626;color:#dc2626;background:#fff;border-radius:5px;font-size:11px;padding:2px 8px;cursor:pointer}'
        var st = document.createElement('style')
        st.textContent = css
        document.head.appendChild(st)
    }

    function openModal() {
        ensureModal()
        document.getElementById('tbFmModal').style.display = 'flex'
        loadList()
    }
    function closeModal() {
        var m = document.getElementById('tbFmModal')
        if (m) m.style.display = 'none'
    }

    function loadList() {
        var $list = $('#tbFmList')
        $list.html('<div class="tb-fm-hint">Memuat...</div>')
        $.ajax({ url: BASE + '/list', type: 'GET', dataType: 'json' })
            .done(function (res) {
                var data = (res && res.data) || []
                $list.empty()
                if (!data.length) {
                    $list.html('<p class="tb-fm-hint">Belum ada file.</p>')
                    return
                }
                data.forEach(function (f) {
                    var item = $(
                        '<div class="tb-fm-item">' +
                        '<img class="tb-fm-thumb" src="' + f.url + '" data-url="' + f.url + '" title="Klik untuk sisipkan">' +
                        '<span class="tb-fm-name">' + (f.name || '') + '</span>' +
                        '<button type="button" class="tb-fm-del" data-key="' + (f.key || f.name) + '"><i class="fas fa-trash-alt"></i> Hapus</button>' +
                        '</div>'
                    )
                    $list.append(item)
                })
            })
            .fail(function () {
                $list.html('<p class="tb-fm-hint" style="color:#dc2626">Gagal memuat daftar file.</p>')
            })
    }

    // --- Event delegation (bound once) ---
    var bound = false
    function bindEvents() {
        if (bound) return
        bound = true

        $(document).on('click', '#tbFmModal .tb-fm-close, #tbFmModal', function (e) {
            if (e.target === this || $(e.target).hasClass('tb-fm-close')) closeModal()
        })

        // Insert image
        $(document).on('click', '.tb-fm-thumb', function () {
            var url = $(this).data('url')
            if (window.trumbowygTarget && url) {
                window.trumbowygTarget.execCmd('insertHTML',
                    '<img src="' + url + '" alt="" style="max-width:100%">')
                window.trumbowygTarget.syncCode()
            }
            closeModal()
        })

        // Delete
        $(document).on('click', '.tb-fm-del', function () {
            if (!confirm('Hapus file ini?')) return
            var $btn = $(this), key = $btn.data('key')
            $.ajax({
                url: BASE + '/delete', type: 'POST', dataType: 'json',
                headers: { 'x-csrf-token': csrfToken() },
                data: { key: key },
            })
                .done(function () { $btn.closest('.tb-fm-item').fadeOut(150, function () { $(this).remove() }) })
                .fail(function (xhr) {
                    var msg = 'Gagal menghapus.'
                    try { msg = JSON.parse(xhr.responseText).message || msg } catch (e) {}
                    alert(msg)
                })
        })

        // Upload
        $(document).on('submit', '#tbFmUploadForm', function (e) {
            e.preventDefault()
            var fd = new FormData(this)
            var $btn = $(this).find('button[type=submit]')
            $btn.prop('disabled', true).text('Mengunggah...')
            $.ajax({
                url: BASE + '/upload', type: 'POST', data: fd,
                processData: false, contentType: false, dataType: 'json',
                headers: { 'x-csrf-token': csrfToken() },
            })
                .done(function () { document.getElementById('tbFmUploadForm').reset(); loadList() })
                .fail(function (xhr) {
                    var msg = 'Gagal mengunggah.'
                    try { msg = JSON.parse(xhr.responseText).message || msg } catch (e) {}
                    alert(msg)
                })
                .always(function () { $btn.prop('disabled', false).text('Upload') })
        })
    }

    // --- Register the Trumbowyg plugin ---
    $.extend(true, $.trumbowyg, {
        langs: { en: { filemanager: 'File Manager' }, id: { filemanager: 'File Manager' } },
        plugins: {
            filemanager: {
                init: function (trumbowyg) {
                    bindEvents()
                    trumbowyg.addBtnDef('filemanager', {
                        fn: function () {
                            window.trumbowygTarget = trumbowyg
                            openModal()
                        },
                        title: trumbowyg.lang.filemanager,
                        ico: 'insertImage',
                        hasIcon: true,
                    })
                },
            },
        },
    })
})(window.jQuery)
