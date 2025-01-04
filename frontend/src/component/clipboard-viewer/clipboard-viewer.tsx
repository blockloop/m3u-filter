import React, {JSX, useCallback, useEffect, useState} from 'react';
import './clipboard-viewer.scss';
import {noop, Observable} from "rxjs";
import copyToClipboard from "../../utils/clipboard";
import {first} from "rxjs/operators";
import {useSnackbar} from "notistack";
import {getIconByName} from "../../icons/icons";

interface ClipboardViewerProps {
    channel: Observable<string>;
}

export default function ClipboardViewer(props: ClipboardViewerProps): JSX.Element {

    const {channel} = props;
    const [data, setData] = useState<string[]>([]);
    const {enqueueSnackbar/*, closeSnackbar*/} = useSnackbar();

    useEffect(() => {
        const sub = channel.subscribe({
            next: (value: string) => setData(d => [...d, value]),
        });
        return () => sub.unsubscribe();
    }, [channel]);

    const handleClear = useCallback(() => {
        setData([]);
    }, []);

    const handleCopyToClipboard = useCallback(() => {
        if (data?.length) {
            copyToClipboard(data.join('\n')).pipe(first()).subscribe({
                next: value => enqueueSnackbar(value ? "Copied to clipboard" : "Copy to clipboard failed!",
                    {variant: value ? 'success' : 'error'}),
                error: err => enqueueSnackbar("Copy to clipboard failed!", {variant: 'error'}),
                complete: noop,
            });
        }
    }, [data, enqueueSnackbar]);

    return <div className={'clipboard-viewer'}>
        <div className={'clipboard-viewer-toolbar'}>
            <button title="Clear All" className={'toolbar-btn'} onClick={handleClear}>{getIconByName('DeleteSweep')}</button>
            <button title="Copy" className={'toolbar-btn'} onClick={handleCopyToClipboard}>{getIconByName('ContentCopy')}</button>
        </div>
        <div className={'clipboard-viewer-content'}>
            <ul>
                {data.map((t, i) => <li key={'text-' + i}>{t}</li>)}
            </ul>
        </div>
    </div>;
}