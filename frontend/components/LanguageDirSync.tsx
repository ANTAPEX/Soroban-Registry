'use client';

import { useEffect } from 'react';
import { i18n } from '@/lib/i18n/client';

const RTL_LANGUAGES = new Set(['ar']);

function applyDir(lng: string) {
    document.documentElement.lang = lng;
    document.documentElement.dir = RTL_LANGUAGES.has(lng) ? 'rtl' : 'ltr';
}

/** Keeps <html lang>/<html dir> in sync with the active i18next language,
 *  wherever it's changed from (Navbar's selector, Settings page, etc.) —
 *  those are only set once at server render otherwise, so switching to
 *  Arabic client-side would translate the text but never flip the page
 *  to RTL. */
export default function LanguageDirSync() {
    useEffect(() => {
        if (i18n.resolvedLanguage) applyDir(i18n.resolvedLanguage);
        const onChange = (lng: string) => applyDir(lng);
        i18n.on('languageChanged', onChange);
        return () => {
            i18n.off('languageChanged', onChange);
        };
    }, []);

    return null;
}
