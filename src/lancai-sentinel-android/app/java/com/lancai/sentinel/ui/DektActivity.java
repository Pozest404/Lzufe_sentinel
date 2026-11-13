package com.lancai.sentinel.ui;

// [OWNER: UI] 应用内第二课堂：WebView 打开用户配置的网址。
// 只有在用户在日程里明确确认「自动报名」后，才在页面加载完成后自动填写并点击报名；
// 普通打开不做任何注入。

import android.app.Activity;
import android.app.AlertDialog;
import android.app.DatePickerDialog;
import android.app.TimePickerDialog;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Typeface;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.MediaStore;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.CookieManager;
import android.webkit.JavascriptInterface;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.DatePicker;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;
import android.widget.TimePicker;

import com.lancai.sentinel.core.CaptureProvider;
import com.lancai.sentinel.core.Config;
import com.lancai.sentinel.core.ReminderScheduler;
import com.lancai.sentinel.core.RustCore;

import java.io.File;
import java.io.InputStream;
import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Locale;

import org.json.JSONObject;

public class DektActivity extends Activity {

    public static final String EXTRA_URL = "url";
    /** 用户已确认自动报名：加载完成后自动填写并点击。 */
    public static final String EXTRA_AUTO_ENROLL = "auto_enroll";

    private WebView web;
    private ProgressBar progress;
    private TextView status;
    private String currentUrl;
    private boolean autoEnroll;
    private boolean injected;
    private final Handler ui = new Handler(Looper.getMainLooper());

    // 扫码签到：站点的「扫码」只认原生桥（wx.scanQRCode 等），WebView 里没有相机能力，
    // 这里用 JS 注入替身 wx 对象 + 原生拍照/相册 → Rust 解码 → 回调页面。
    private static final String JS_BRIDGE_NAME = "LancaiBridge";
    private static final int REQ_SCAN_CAMERA = 3101;
    private static final int REQ_SCAN_GALLERY = 3102;
    private Uri cameraScanUri;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        String url = null;
        if (getIntent() != null) {
            url = getIntent().getStringExtra(EXTRA_URL);
            autoEnroll = getIntent().getBooleanExtra(EXTRA_AUTO_ENROLL, false);
        }
        if (TextUtils.isEmpty(url)) url = Config.get(this).getDektUrl();
        currentUrl = url;
        render();
        web.loadUrl(url);
    }

    private void render() {
        LinearLayout page = UiKit.vertical(this);
        page.setBackgroundColor(UiKit.BG);

        LinearLayout bar = UiKit.topBar(this, autoEnroll ? "第二课堂 · 自动报名" : "第二课堂");

        TextView refresh = UiKit.text(this, "刷新", 13, 0xFF4E4E5A);
        refresh.setTypeface(null, Typeface.BOLD);
        refresh.setPadding(dp(10), dp(8), dp(8), dp(8));
        refresh.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                if (web != null) {
                    web.reload();
                }
            }
        });
        bar.addView(refresh);

        TextView addToTable = UiKit.text(this, "记到课表", 13, 0xFF4E4E5A);
        addToTable.setTypeface(null, Typeface.BOLD);
        addToTable.setPadding(dp(6), dp(8), dp(10), dp(8));
        addToTable.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                openQuickAdd();
            }
        });
        bar.addView(addToTable);
        page.addView(bar, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        progress = new ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal);
        progress.setMax(100);
        progress.setProgress(0);
        LinearLayout.LayoutParams pp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(3));
        page.addView(progress, pp);

        status = UiKit.text(this, "", 11.5f, UiKit.SUB);
        status.setPadding(dp(14), dp(4), dp(14), dp(2));
        status.setVisibility(View.GONE);
        page.addView(status);

        web = new WebView(this);
        web.setBackgroundColor(0xFFFFFFFF);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setDatabaseEnabled(true);
        s.setLoadWithOverviewMode(true);
        s.setUseWideViewPort(true);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        s.setJavaScriptCanOpenWindowsAutomatically(true);
        s.setSupportMultipleWindows(false);
        if (Build.VERSION.SDK_INT >= 21) {
            s.setMixedContentMode(WebSettings.MIXED_CONTENT_COMPATIBILITY_MODE);
            try {
                CookieManager.getInstance().setAcceptThirdPartyCookies(web, true);
            } catch (Throwable ignore) {
            }
        }
        CookieManager.getInstance().setAcceptCookie(true);

        web.addJavascriptInterface(new ScanBridge(), JS_BRIDGE_NAME);

        web.setWebChromeClient(new WebChromeClient() {
            @Override
            public void onProgressChanged(WebView view, int newProgress) {
                if (progress == null) return;
                progress.setProgress(newProgress);
                progress.setVisibility(newProgress >= 100 ? View.GONE : View.VISIBLE);
            }
        });

        web.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                if (request == null || request.getUrl() == null) return false;
                return handleUrl(request.getUrl().toString());
            }

            @SuppressWarnings("deprecation")
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, String url) {
                return handleUrl(url);
            }

            @Override
            public void onPageFinished(WebView view, String url) {
                currentUrl = url;
                if (progress != null) progress.setVisibility(View.GONE);
                if (status != null) status.setVisibility(View.GONE);
                installScanShim();
                if (autoEnroll && !injected) {
                    injected = true;
                    ui.postDelayed(new Runnable() {
                        @Override
                        public void run() {
                            runAutoEnroll();
                        }
                    }, 1200);
                }
            }

            @Override
            public void onReceivedError(WebView view, int errorCode, String description,
                                        String failingUrl) {
                showStatus("加载失败：" + (description == null ? "网络错误" : description)
                        + "，可点右上角刷新或浏览器打开");
            }

            @Override
            public void onReceivedError(WebView view, WebResourceRequest request,
                                        android.webkit.WebResourceError error) {
                if (request != null && request.isForMainFrame() && error != null) {
                    showStatus("加载失败：" + error.getDescription()
                            + "，可点右上角刷新或浏览器打开");
                }
            }
        });

        page.addView(web, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        setContentView(page);
    }

    // ---------- 扫码签到 ----------

    /** JS 侧入口（运行在 WebView 的 JS 线程，必须切回 UI 线程）。 */
    private final class ScanBridge {
        @JavascriptInterface
        public void startScan(final String mode) {
            ui.post(new Runnable() {
                @Override
                public void run() {
                    beginScan(mode);
                }
            });
        }
    }

    /**
     * 注入 wx 替身：站点的扫码逻辑只认原生桥（wx.scanQRCode / dd.biz.util.scan / doschool），
     * WebView 里没有相机能力，点「扫码」原本毫无反应。这里接管 wx.scanQRCode，
     * 扫描结果按 WeChat JSSDK 的回调格式回灌给页面，页面自己的签到请求不受影响。
     */
    private void installScanShim() {
        if (web == null) return;
        String js = "(function(){try{"
                + "try{localStorage.setItem('platform','notInDingTalk')}catch(e){}"
                + "var w=window.wx||{};"
                + "w.config=function(){return w};"
                + "w.ready=function(cb){try{if(cb)cb()}catch(e){}return w};"
                + "w.error=function(){return w};"
                + "w.checkJsApi=function(o){try{if(o&&o.success)o.success({checkResult:{}})}catch(e){}return w};"
                + "w.scanQRCode=function(o){window.__lancaiScanOpts=o||{};try{"
                + JS_BRIDGE_NAME + ".startScan('wx')}catch(e){}return w};"
                + "window.wx=w;"
                + "window.__lancaiScanOk=function(t){var o=window.__lancaiScanOpts;"
                + "if(o&&o.success){o.success({errMsg:'scanQRCode:ok',resultStr:t,scanType:'qrCode'})}"
                + "else if(o&&o.complete){o.complete({errMsg:'scanQRCode:ok',resultStr:t})}};"
                + "window.__lancaiScanFail=function(m){var o=window.__lancaiScanOpts;"
                + "if(o&&o.cancel){o.cancel({errMsg:'scanQRCode:cancel'})}"
                + "else if(o&&o.fail){o.fail({errMsg:'scanQRCode:fail',errMsgDetail:m||''})}};"
                + "}catch(e){}})();";
        web.evaluateJavascript(js, null);
    }

    private void beginScan(String mode) {
        final String[] items = new String[]{"拍照扫码", "从相册选择"};
        new AlertDialog.Builder(this)
                .setTitle("扫码签到")
                .setItems(items, new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface dialog, int which) {
                        if (which == 0) launchCamera();
                        else launchGallery();
                    }
                })
                .setNegativeButton("取消", new DialogInterface.OnClickListener() {
                    @Override
                    public void onClick(DialogInterface dialog, int which) {
                        notifyScanFail("已取消");
                    }
                })
                .setOnCancelListener(new DialogInterface.OnCancelListener() {
                    @Override
                    public void onCancel(DialogInterface dialog) {
                        notifyScanFail("已取消");
                    }
                })
                .show();
    }

    private void launchCamera() {
        try {
            String name = "scan_" + System.currentTimeMillis() + ".jpg";
            File f = CaptureProvider.fileFor(this, name);
            File dir = f.getParentFile();
            if (dir != null && !dir.exists()) dir.mkdirs();
            cameraScanUri = CaptureProvider.uriFor(this, name);
            Intent i = new Intent(MediaStore.ACTION_IMAGE_CAPTURE);
            i.putExtra(MediaStore.EXTRA_OUTPUT, cameraScanUri);
            i.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_READ_URI_PERMISSION);
            if (i.resolveActivity(getPackageManager()) == null) {
                launchGallery();
                return;
            }
            startActivityForResult(i, REQ_SCAN_CAMERA);
        } catch (Throwable t) {
            launchGallery();
        }
    }

    private void launchGallery() {
        try {
            Intent i = new Intent(Intent.ACTION_GET_CONTENT);
            i.setType("image/*");
            i.addCategory(Intent.CATEGORY_OPENABLE);
            startActivityForResult(Intent.createChooser(i, "选择二维码图片"), REQ_SCAN_GALLERY);
        } catch (Throwable t) {
            notifyScanFail("无法打开图片选择器");
        }
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_SCAN_CAMERA && requestCode != REQ_SCAN_GALLERY) return;
        if (resultCode != RESULT_OK) {
            notifyScanFail("已取消");
            return;
        }
        Uri uri = requestCode == REQ_SCAN_CAMERA
                ? cameraScanUri
                : (data == null ? null : data.getData());
        if (uri == null) {
            notifyScanFail("没有拿到图片");
            return;
        }
        decodeQrAsync(uri);
    }

    /** 图片解码 + 灰度转换在子线程，二维码识别由 Rust 完成。 */
    private void decodeQrAsync(final Uri uri) {
        showStatus("正在识别二维码…");
        new Thread(new Runnable() {
            @Override
            public void run() {
                String text = null;
                String err = null;
                try {
                    Bitmap bmp = loadBitmap(uri, 1600);
                    if (bmp == null) {
                        err = "图片读取失败";
                    } else {
                        int w = bmp.getWidth();
                        int h = bmp.getHeight();
                        int[] px = new int[w * h];
                        bmp.getPixels(px, 0, w, 0, 0, w, h);
                        bmp.recycle();
                        byte[] gray = new byte[w * h];
                        for (int i = 0; i < px.length; i++) {
                            int p = px[i];
                            int r = (p >> 16) & 0xFF;
                            int g = (p >> 8) & 0xFF;
                            int b = p & 0xFF;
                            gray[i] = (byte) ((r * 299 + g * 587 + b * 114) / 1000);
                        }
                        text = RustCore.decodeQr(w, h, gray);
                        if (text == null) err = "未识别到二维码，请靠近后重拍";
                    }
                } catch (Throwable t) {
                    err = "识别失败：" + t.getClass().getSimpleName();
                }
                final String okText = text;
                final String failMsg = err;
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        if (okText != null) {
                            showStatus("已识别二维码：" + shorten(okText));
                            deliverScanResult(okText);
                        } else {
                            notifyScanFail(failMsg == null ? "识别失败" : failMsg);
                        }
                    }
                });
            }
        }).start();
    }

    private Bitmap loadBitmap(Uri uri, int maxSide) throws Exception {
        BitmapFactory.Options bounds = new BitmapFactory.Options();
        bounds.inJustDecodeBounds = true;
        InputStream in = getContentResolver().openInputStream(uri);
        try {
            BitmapFactory.decodeStream(in, null, bounds);
        } finally {
            closeQuietly(in);
        }
        int sample = 1;
        int side = Math.max(bounds.outWidth, bounds.outHeight);
        while (side / sample > maxSide) sample *= 2;
        BitmapFactory.Options opts = new BitmapFactory.Options();
        opts.inSampleSize = sample;
        opts.inPreferredConfig = Bitmap.Config.ARGB_8888;
        in = getContentResolver().openInputStream(uri);
        try {
            return BitmapFactory.decodeStream(in, null, opts);
        } finally {
            closeQuietly(in);
        }
    }

    private static void closeQuietly(InputStream in) {
        if (in != null) {
            try {
                in.close();
            } catch (Throwable ignore) {
            }
        }
    }

    private void deliverScanResult(String text) {
        if (web == null || text == null) return;
        web.evaluateJavascript("if(window.__lancaiScanOk){window.__lancaiScanOk("
                + JSONObject.quote(text) + ")}", null);
    }

    private void notifyScanFail(String msg) {
        String safe = msg == null ? "" : msg;
        if (web != null) {
            web.evaluateJavascript("if(window.__lancaiScanFail){window.__lancaiScanFail("
                    + JSONObject.quote(safe) + ")}", null);
        }
        showStatus("扫码失败：" + (safe.isEmpty() ? "未知原因" : safe));
    }

    private static String shorten(String s) {
        if (s == null) return "";
        return s.length() <= 48 ? s : s.substring(0, 48) + "…";
    }

    /** 把当前第二课堂活动记到课表：时间等解析全部交给 Rust。 */
    private void openQuickAdd() {
        final Calendar def = Calendar.getInstance();
        def.set(Calendar.HOUR_OF_DAY, 19);
        def.set(Calendar.MINUTE, 0);
        def.set(Calendar.SECOND, 0);
        def.set(Calendar.MILLISECOND, 0);
        final long[] picked = { def.getTimeInMillis() };

        LinearLayout form = UiKit.vertical(this);
        form.setPadding(dp(18), dp(4), dp(18), dp(2));

        final EditText etTitle = input("例：志愿服务讲座");
        final EditText etPlace = input("可选，例：大学生活动中心");
        final TextView dateRow = pickRow();
        final TextView timeRow = pickRow();
        final TextView clearTime = UiKit.text(this, "不设时间（只进日程列表）", 12, UiKit.VIOLET);
        clearTime.setPadding(0, dp(8), 0, dp(2));

        updateWhenRows(dateRow, timeRow, picked[0]);

        dateRow.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                Calendar c = Calendar.getInstance();
                if (picked[0] > 0) c.setTimeInMillis(picked[0]);
                new DatePickerDialog(DektActivity.this,
                        new DatePickerDialog.OnDateSetListener() {
                            @Override
                            public void onDateSet(DatePicker view, int year, int month, int day) {
                                Calendar c2 = Calendar.getInstance();
                                if (picked[0] > 0) c2.setTimeInMillis(picked[0]);
                                c2.set(Calendar.YEAR, year);
                                c2.set(Calendar.MONTH, month);
                                c2.set(Calendar.DAY_OF_MONTH, day);
                                picked[0] = c2.getTimeInMillis();
                                updateWhenRows(dateRow, timeRow, picked[0]);
                            }
                        }, c.get(Calendar.YEAR), c.get(Calendar.MONTH),
                        c.get(Calendar.DAY_OF_MONTH)).show();
            }
        });
        timeRow.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                Calendar c = Calendar.getInstance();
                if (picked[0] > 0) c.setTimeInMillis(picked[0]);
                new TimePickerDialog(DektActivity.this,
                        new TimePickerDialog.OnTimeSetListener() {
                            @Override
                            public void onTimeSet(TimePicker view, int hour, int minute) {
                                Calendar c2 = Calendar.getInstance();
                                if (picked[0] > 0) c2.setTimeInMillis(picked[0]);
                                c2.set(Calendar.HOUR_OF_DAY, hour);
                                c2.set(Calendar.MINUTE, minute);
                                picked[0] = c2.getTimeInMillis();
                                updateWhenRows(dateRow, timeRow, picked[0]);
                            }
                        }, c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE), true).show();
            }
        });
        clearTime.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                picked[0] = 0;
                updateWhenRows(dateRow, timeRow, 0);
            }
        });

        form.addView(UiKit.label(this, "活动名称"));
        form.addView(etTitle);
        form.addView(UiKit.label(this, "日期"));
        form.addView(dateRow);
        form.addView(UiKit.label(this, "开始时间"));
        form.addView(timeRow);
        form.addView(clearTime);
        form.addView(UiKit.label(this, "地点"));
        form.addView(etPlace);
        form.addView(UiKit.hint(this,
                "保存后会出现在对应周的课表上；点「不设时间」则只进日程列表、不上课表。"));

        final AlertDialog d = new AlertDialog.Builder(this)
                .setTitle("记到课表")
                .setView(form)
                .setNegativeButton("取消", null)
                .setPositiveButton("保存", null)
                .create();
        d.setOnShowListener(new DialogInterface.OnShowListener() {
            @Override
            public void onShow(DialogInterface dialog) {
                ((AlertDialog) dialog).getButton(AlertDialog.BUTTON_POSITIVE)
                        .setOnClickListener(new View.OnClickListener() {
                            @Override
                            public void onClick(View v) {
                                saveQuickAdd(d, etTitle, etPlace, whenString(picked[0]));
                            }
                        });
            }
        });
        d.show();
    }

    private TextView pickRow() {
        TextView t = UiKit.text(this, "", 14, UiKit.INK);
        t.setPadding(dp(12), dp(11), dp(12), dp(11));
        UiKit.rounded(t, this, 0xFFF4F6F6, 0, 10);
        t.setLayoutParams(new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return t;
    }

    private void updateWhenRows(TextView dateRow, TextView timeRow, long ms) {
        if (ms <= 0) {
            dateRow.setText("点此选择日期");
            timeRow.setText("点此选择时间");
            dateRow.setTextColor(UiKit.MUTED);
            timeRow.setTextColor(UiKit.MUTED);
            return;
        }
        Calendar c = Calendar.getInstance();
        c.setTimeInMillis(ms);
        dateRow.setText(String.format(Locale.CHINA, "%d月%d日 周%s",
                c.get(Calendar.MONTH) + 1, c.get(Calendar.DAY_OF_MONTH),
                "一二三四五六日".charAt((c.get(Calendar.DAY_OF_WEEK) + 5) % 7)));
        timeRow.setText(String.format(Locale.CHINA, "%02d:%02d",
                c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE)));
        dateRow.setTextColor(UiKit.INK);
        timeRow.setTextColor(UiKit.INK);
    }

    private String whenString(long ms) {
        if (ms <= 0) return "";
        return new SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.CHINA)
                .format(new Date(ms));
    }

    private void saveQuickAdd(final AlertDialog d, EditText etTitle, EditText etPlace,
                              final String when) {
        final String title = etTitle.getText().toString().trim();
        if (title.isEmpty()) {
            UiKit.toast(this, "请填写活动名称");
            return;
        }
        final String place = etPlace.getText().toString().trim();
        final String url = currentUrl == null ? "" : currentUrl;
        final Context ctx = this;
        new Thread(new Runnable() {
            @Override
            public void run() {
                String error = null;
                try {
                    JSONObject o = new JSONObject();
                    o.put("title", title);
                    o.put("due", when);
                    o.put("note", place.isEmpty() ? "" : "地点：" + place);
                    o.put("kind", "activity");
                    o.put("url", url);
                    o.put("remind_min", 10);
                    String res = RustCore.scheduleSave(ctx, o.toString());
                    JSONObject r = res == null ? null : new JSONObject(res);
                    if (r != null && r.has("ok") && !r.optBoolean("ok")) {
                        error = r.optString("error", "保存失败");
                    }
                } catch (Throwable t) {
                    error = t.getMessage();
                }
                final String err = error;
                ui.post(new Runnable() {
                    @Override
                    public void run() {
                        if (err != null) {
                            UiKit.toast(DektActivity.this, err);
                            return;
                        }
                        d.dismiss();
                        UiKit.toast(DektActivity.this, "已记到课表");
                        new Thread(new Runnable() {
                            @Override
                            public void run() {
                                ReminderScheduler.reschedule(ctx);
                            }
                        }, "lancai-dekt-remind").start();
                    }
                });
            }
        }, "lancai-dekt-save").start();
    }

    private EditText input(String hint) {
        EditText e = new EditText(this);
        e.setHint(hint);
        e.setHintTextColor(UiKit.MUTED);
        e.setTextColor(UiKit.INK);
        e.setTextSize(14);
        e.setSingleLine(true);
        e.setBackgroundColor(0xFFF4F6F6);
        e.setPadding(dp(12), dp(10), dp(12), dp(10));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.bottomMargin = dp(4);
        e.setLayoutParams(lp);
        return e;
    }

    /** 自动报名：按常见字段名填写姓名/学号/手机号，再点击含「报名」的按钮。 */
    private void runAutoEnroll() {
        if (web == null) return;
        Config cfg = Config.get(this);
        String name = JSONObject.quote(cfg.getProfileName());
        String sid = JSONObject.quote(cfg.getProfileSid());
        String phone = JSONObject.quote(cfg.getProfilePhone());
        String js =
                "(function(){"
                        + "var d={name:" + name + ",sid:" + sid + ",phone:" + phone + "};"
                        + "function key(e){return ((e.getAttribute('name')||'')+' '"
                        + "+(e.getAttribute('id')||'')+' '+(e.getAttribute('placeholder')||'')+' '"
                        + "+(e.getAttribute('aria-label')||'')).toLowerCase();}"
                        + "function fill(e,v){if(!e||!v||(e.value&&e.value.trim()))return false;"
                        + "e.focus();e.value=v;"
                        + "e.dispatchEvent(new Event('input',{bubbles:true}));"
                        + "e.dispatchEvent(new Event('change',{bubbles:true}));return true;}"
                        + "var ins=document.querySelectorAll('input:not([type=hidden]),textarea');"
                        + "var u={n:false,s:false,p:false};"
                        + "for(var i=0;i<ins.length;i++){var e=ins[i],k=key(e);"
                        + "if(!u.p&&/手机|电话|mobile|phone|tel/.test(k)){if(fill(e,d.phone))u.p=true;continue;}"
                        + "if(!u.s&&/学号|student|sid|number/.test(k)){if(fill(e,d.sid))u.s=true;continue;}"
                        + "if(!u.n&&/姓名|名字|name|realname/.test(k)){if(fill(e,d.name))u.n=true;}}"
                        + "var ns=document.querySelectorAll('button,a,input[type=button],"
                        + "input[type=submit],[role=button],.btn,.button');"
                        + "for(var j=0;j<ns.length;j++){"
                        + "var t=(ns[j].innerText||ns[j].value||'').replace(/\\s+/g,'');"
                        + "if(t.indexOf('报名')>=0&&t.indexOf('取消')<0&&t.indexOf('已报名')<0){"
                        + "ns[j].click();return '已自动填写并点击：'+t;}}"
                        + "return (u.n||u.s||u.p)?'已填写信息，请手动点击报名':'未找到报名按钮，请手动操作';"
                        + "})()";
        showStatus("自动报名：正在填写报名信息…");
        try {
            web.evaluateJavascript(js, new ValueCallback<String>() {
                @Override
                public void onReceiveValue(String value) {
                    String msg = value == null ? "" : value.replace("\"", "").trim();
                    if (TextUtils.isEmpty(msg) || "null".equals(msg)) {
                        msg = "已尝试自动报名，请在页面确认结果";
                    }
                    showStatus("自动报名：" + msg);
                }
            });
        } catch (Throwable t) {
            showStatus("自动报名失败，请手动操作");
        }
    }

    /** 仅 http/https 留在应用内，其他 scheme 交给系统。 */
    private boolean handleUrl(String url) {
        if (TextUtils.isEmpty(url)) return false;
        String lower = url.toLowerCase();
        if (lower.startsWith("http://") || lower.startsWith("https://")) {
            return false;
        }
        openExternal(url);
        return true;
    }

    private void openExternal(String url) {
        if (TextUtils.isEmpty(url)) return;
        try {
            Intent i = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
            i.addCategory(Intent.CATEGORY_BROWSABLE);
            startActivity(i);
        } catch (Throwable t) {
            UiKit.toastLong(this, "没有可打开该链接的应用");
        }
    }

    private void showStatus(String text) {
        if (status == null) return;
        status.setText(text);
        status.setVisibility(View.VISIBLE);
    }

    @Override
    public void onBackPressed() {
        if (web != null && web.canGoBack()) {
            web.goBack();
        } else {
            super.onBackPressed();
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (web != null) {
            web.resumeTimers();
            web.onResume();
        }
    }

    @Override
    protected void onPause() {
        if (web != null) {
            web.onPause();
            // 停掉全局 JS 定时器（setTimeout/setInterval），防止后台继续耗电。
            // onPause() 只暂停 WebView 内部动画，pauseTimers() 才是全局停转。
            web.pauseTimers();
        }
        super.onPause();
    }

    @Override
    protected void onDestroy() {
        if (web != null) {
            try {
                web.stopLoading();
                web.setWebViewClient(null);
                web.destroy();
            } catch (Throwable ignore) {
            }
            web = null;
        }
        super.onDestroy();
    }

    private int dp(float v) {
        return UiKit.dp(this, v);
    }
}
