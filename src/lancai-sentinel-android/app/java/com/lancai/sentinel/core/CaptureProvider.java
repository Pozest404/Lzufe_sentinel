package com.lancai.sentinel.core;

// [OWNER: CORE] 极简 ContentProvider：只为 ACTION_IMAGE_CAPTURE 提供一个可写的 content:// URI。
// 不用 AndroidX 的 FileProvider，保持零第三方依赖。

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;

import java.io.File;
import java.io.FileNotFoundException;

public class CaptureProvider extends ContentProvider {

    public static final String AUTHORITY = "com.lancai.sentinel.capture";

    public static File captureDir(Context c) {
        File dir = new File(c.getCacheDir(), "capture");
        if (!dir.exists()) dir.mkdirs();
        return dir;
    }

    public static Uri uriFor(Context c, String name) {
        return Uri.parse("content://" + AUTHORITY + "/" + name);
    }

    public static File fileFor(Context c, String name) {
        return new File(captureDir(c), name);
    }

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        String name = uri == null ? null : uri.getLastPathSegment();
        if (name == null || name.isEmpty() || name.contains("/") || name.contains("..")) {
            throw new FileNotFoundException("非法文件名");
        }
        File f = fileFor(getContext(), name);
        int flags = ParcelFileDescriptor.MODE_READ_ONLY;
        if (mode != null && mode.contains("w")) {
            flags = ParcelFileDescriptor.MODE_CREATE
                    | ParcelFileDescriptor.MODE_WRITE_ONLY
                    | ParcelFileDescriptor.MODE_TRUNCATE;
        }
        return ParcelFileDescriptor.open(f, flags);
    }

    @Override
    public String getType(Uri uri) {
        return "image/jpeg";
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection,
                        String[] selectionArgs, String sortOrder) {
        return null;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        return null;
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        return 0;
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        return 0;
    }
}
