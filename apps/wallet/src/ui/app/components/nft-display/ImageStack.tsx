// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { getObjectDisplay, type SuiObjectResponse } from '@mysten/sui.js';

import { Text } from '../../shared/text';
import { NftImage } from './NftImage';

export function ImageStack({ objects }: { objects: SuiObjectResponse[] }) {
	return (
		<div>
			{objects.map((object, idx) => {
				return (
					<>
						<div
							className={`transition-all ease-ease-out-cubic shadow-md rounded-md absolute h-24 w-24 group-hover:shadow-blurXl group-hover:shadow-steel/50 group-hover:scale-75 ${
								styles[idx] ?? ''
							} `}
							style={{
								zIndex: objects.length - idx,
								top: `-${idx * 6}px`,
							}}
						>
							<NftImage name="Kiosk" src={getObjectDisplay(object)?.data?.image_url!} size="lg" />
						</div>
					</>
				);
			})}
		</div>
	);
}
