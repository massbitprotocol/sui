// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useGetKioskContents } from '@mysten/core';
import { type SuiObjectResponse } from '@mysten/sui.js';

import { useActiveAddress } from '../../hooks';
import { NftImage } from './NftImage';

export function Kiosk({ object }: { object?: SuiObjectResponse }) {
	const address = useActiveAddress();

	const { data: kioskData } = useGetKioskContents(address);
	const kioskId =
		typeof object?.data?.content === 'object' && 'fields' in object?.data?.content
			? object?.data?.content?.fields.for
			: null;
	const suiKiosk = kioskData?.kiosks.sui.get(kioskId);

	return (
		<div className="relative bg-hero">
			{suiKiosk?.items.reverse().map((image, idx) => (
				<div
					className={`absolute shadow-md rounded-md`}
					style={{ zIndex: suiKiosk.items.length - idx, top: `${idx * 5}px`, left: `${idx * 5}px` }}
				>
					<NftImage name="" src={image.data?.display?.data.image_url} size={'md'} />
					{/* // <img src={image.data?.display?.data.image_url} className="h-5 w-5" /> */}
				</div>
			))}
		</div>
	);
}
